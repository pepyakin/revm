//! Osaka-specialized match-based opcode dispatch loop.
//!
//! Replaces the indirect function-pointer table dispatch with a flat `match opcode { ... }`
//! to enable LLVM threaded-dispatch optimization (each handler tail-branches to the next
//! opcode fetch).

use crate::instructions::{
    arithmetic, bitwise, block_info, contract, control, host as host_instr, memory, stack, system,
    tx_info,
};
use crate::interpreter::{EthInterpreter, Interpreter};
use crate::interpreter_types::{Jumps, LoopControl};
use crate::{Host, InstructionContext, InstructionTable, InterpreterAction};
use bytecode::opcode::*;

/// Osaka-only interpreter loop using match-based dispatch instead of function-pointer table.
///
/// The entire fetch → gas → match → execute loop lives in this single function so LLVM can
/// form a threaded-dispatch pattern where each handler ends with a branch back to the top
/// of the loop (or even a direct jump to the next handler).
#[inline(never)]
pub fn run_osaka_match<EXT, H: Host + ?Sized>(
    interp: &mut Interpreter<EthInterpreter<EXT>>,
    instruction_table: &InstructionTable<EthInterpreter<EXT>, H>,
    host: &mut H,
) -> InterpreterAction {
    while interp.bytecode.is_not_end() {
        let opcode = interp.bytecode.opcode();

        // Advance PC past the opcode byte — matches existing `step` ordering exactly.
        interp.bytecode.relative_jump(1);

        // Charge static gas from the existing Osaka instruction table.
        let static_gas = unsafe { instruction_table.get_unchecked(opcode as usize) }.static_gas();
        if interp.gas.record_cost_unsafe(static_gas) {
            interp.halt_oog();
            continue;
        }

        // Direct-call dispatch. LLVM sees concrete call targets for every arm, enabling
        // inlining of hot/small handlers and threaded-dispatch code layout.
        match opcode {
            // --- Arithmetic (0x00-0x0B) ---
            STOP => control::stop(InstructionContext {
                interpreter: interp,
                host,
            }),
            ADD => arithmetic::add(InstructionContext {
                interpreter: interp,
                host,
            }),
            MUL => arithmetic::mul(InstructionContext {
                interpreter: interp,
                host,
            }),
            SUB => arithmetic::sub(InstructionContext {
                interpreter: interp,
                host,
            }),
            DIV => arithmetic::div(InstructionContext {
                interpreter: interp,
                host,
            }),
            SDIV => arithmetic::sdiv(InstructionContext {
                interpreter: interp,
                host,
            }),
            MOD => arithmetic::rem(InstructionContext {
                interpreter: interp,
                host,
            }),
            SMOD => arithmetic::smod(InstructionContext {
                interpreter: interp,
                host,
            }),
            ADDMOD => arithmetic::addmod(InstructionContext {
                interpreter: interp,
                host,
            }),
            MULMOD => arithmetic::mulmod(InstructionContext {
                interpreter: interp,
                host,
            }),
            EXP => arithmetic::exp(InstructionContext {
                interpreter: interp,
                host,
            }),
            SIGNEXTEND => arithmetic::signextend(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Comparison & Bitwise (0x10-0x1E) ---
            LT => bitwise::lt(InstructionContext {
                interpreter: interp,
                host,
            }),
            GT => bitwise::gt(InstructionContext {
                interpreter: interp,
                host,
            }),
            SLT => bitwise::slt(InstructionContext {
                interpreter: interp,
                host,
            }),
            SGT => bitwise::sgt(InstructionContext {
                interpreter: interp,
                host,
            }),
            EQ => bitwise::eq(InstructionContext {
                interpreter: interp,
                host,
            }),
            ISZERO => bitwise::iszero(InstructionContext {
                interpreter: interp,
                host,
            }),
            AND => bitwise::bitand(InstructionContext {
                interpreter: interp,
                host,
            }),
            OR => bitwise::bitor(InstructionContext {
                interpreter: interp,
                host,
            }),
            XOR => bitwise::bitxor(InstructionContext {
                interpreter: interp,
                host,
            }),
            NOT => bitwise::not(InstructionContext {
                interpreter: interp,
                host,
            }),
            BYTE => bitwise::byte(InstructionContext {
                interpreter: interp,
                host,
            }),
            SHL => bitwise::shl(InstructionContext {
                interpreter: interp,
                host,
            }),
            SHR => bitwise::shr(InstructionContext {
                interpreter: interp,
                host,
            }),
            SAR => bitwise::sar(InstructionContext {
                interpreter: interp,
                host,
            }),
            CLZ => bitwise::clz(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Keccak (0x20) ---
            KECCAK256 => system::keccak256(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Environmental (0x30-0x3F) ---
            ADDRESS => system::address(InstructionContext {
                interpreter: interp,
                host,
            }),
            BALANCE => host_instr::balance(InstructionContext {
                interpreter: interp,
                host,
            }),
            ORIGIN => tx_info::origin(InstructionContext {
                interpreter: interp,
                host,
            }),
            CALLER => system::caller(InstructionContext {
                interpreter: interp,
                host,
            }),
            CALLVALUE => system::callvalue(InstructionContext {
                interpreter: interp,
                host,
            }),
            CALLDATALOAD => system::calldataload(InstructionContext {
                interpreter: interp,
                host,
            }),
            CALLDATASIZE => system::calldatasize(InstructionContext {
                interpreter: interp,
                host,
            }),
            CALLDATACOPY => system::calldatacopy(InstructionContext {
                interpreter: interp,
                host,
            }),
            CODESIZE => system::codesize(InstructionContext {
                interpreter: interp,
                host,
            }),
            CODECOPY => system::codecopy(InstructionContext {
                interpreter: interp,
                host,
            }),
            GASPRICE => tx_info::gasprice(InstructionContext {
                interpreter: interp,
                host,
            }),
            EXTCODESIZE => host_instr::extcodesize(InstructionContext {
                interpreter: interp,
                host,
            }),
            EXTCODECOPY => host_instr::extcodecopy(InstructionContext {
                interpreter: interp,
                host,
            }),
            RETURNDATASIZE => system::returndatasize(InstructionContext {
                interpreter: interp,
                host,
            }),
            RETURNDATACOPY => system::returndatacopy(InstructionContext {
                interpreter: interp,
                host,
            }),
            EXTCODEHASH => host_instr::extcodehash(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Block info (0x40-0x4B) ---
            BLOCKHASH => host_instr::blockhash(InstructionContext {
                interpreter: interp,
                host,
            }),
            COINBASE => block_info::coinbase(InstructionContext {
                interpreter: interp,
                host,
            }),
            TIMESTAMP => block_info::timestamp(InstructionContext {
                interpreter: interp,
                host,
            }),
            NUMBER => block_info::block_number(InstructionContext {
                interpreter: interp,
                host,
            }),
            DIFFICULTY => block_info::difficulty(InstructionContext {
                interpreter: interp,
                host,
            }),
            GASLIMIT => block_info::gaslimit(InstructionContext {
                interpreter: interp,
                host,
            }),
            CHAINID => block_info::chainid(InstructionContext {
                interpreter: interp,
                host,
            }),
            SELFBALANCE => host_instr::selfbalance(InstructionContext {
                interpreter: interp,
                host,
            }),
            BASEFEE => block_info::basefee(InstructionContext {
                interpreter: interp,
                host,
            }),
            BLOBHASH => tx_info::blob_hash(InstructionContext {
                interpreter: interp,
                host,
            }),
            BLOBBASEFEE => block_info::blob_basefee(InstructionContext {
                interpreter: interp,
                host,
            }),
            SLOTNUM => block_info::slot_num(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Stack/Memory/Storage (0x50-0x5E) ---
            POP => stack::pop(InstructionContext {
                interpreter: interp,
                host,
            }),
            MLOAD => memory::mload(InstructionContext {
                interpreter: interp,
                host,
            }),
            MSTORE => memory::mstore(InstructionContext {
                interpreter: interp,
                host,
            }),
            MSTORE8 => memory::mstore8(InstructionContext {
                interpreter: interp,
                host,
            }),
            SLOAD => host_instr::sload(InstructionContext {
                interpreter: interp,
                host,
            }),
            SSTORE => host_instr::sstore(InstructionContext {
                interpreter: interp,
                host,
            }),
            JUMP => control::jump(InstructionContext {
                interpreter: interp,
                host,
            }),
            JUMPI => control::jumpi(InstructionContext {
                interpreter: interp,
                host,
            }),
            PC => control::pc(InstructionContext {
                interpreter: interp,
                host,
            }),
            MSIZE => memory::msize(InstructionContext {
                interpreter: interp,
                host,
            }),
            GAS => system::gas(InstructionContext {
                interpreter: interp,
                host,
            }),
            JUMPDEST => control::jumpdest(InstructionContext {
                interpreter: interp,
                host,
            }),
            TLOAD => host_instr::tload(InstructionContext {
                interpreter: interp,
                host,
            }),
            TSTORE => host_instr::tstore(InstructionContext {
                interpreter: interp,
                host,
            }),
            MCOPY => memory::mcopy(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Push (0x5F-0x7F) ---
            PUSH0 => stack::push0(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH1 => stack::push::<1, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH2 => stack::push::<2, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH3 => stack::push::<3, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH4 => stack::push::<4, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH5 => stack::push::<5, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH6 => stack::push::<6, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH7 => stack::push::<7, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH8 => stack::push::<8, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH9 => stack::push::<9, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH10 => stack::push::<10, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH11 => stack::push::<11, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH12 => stack::push::<12, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH13 => stack::push::<13, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH14 => stack::push::<14, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH15 => stack::push::<15, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH16 => stack::push::<16, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH17 => stack::push::<17, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH18 => stack::push::<18, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH19 => stack::push::<19, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH20 => stack::push::<20, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH21 => stack::push::<21, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH22 => stack::push::<22, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH23 => stack::push::<23, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH24 => stack::push::<24, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH25 => stack::push::<25, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH26 => stack::push::<26, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH27 => stack::push::<27, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH28 => stack::push::<28, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH29 => stack::push::<29, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH30 => stack::push::<30, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH31 => stack::push::<31, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            PUSH32 => stack::push::<32, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Dup (0x80-0x8F) ---
            DUP1 => stack::dup::<1, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP2 => stack::dup::<2, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP3 => stack::dup::<3, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP4 => stack::dup::<4, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP5 => stack::dup::<5, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP6 => stack::dup::<6, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP7 => stack::dup::<7, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP8 => stack::dup::<8, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP9 => stack::dup::<9, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP10 => stack::dup::<10, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP11 => stack::dup::<11, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP12 => stack::dup::<12, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP13 => stack::dup::<13, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP14 => stack::dup::<14, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP15 => stack::dup::<15, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            DUP16 => stack::dup::<16, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Swap (0x90-0x9F) ---
            SWAP1 => stack::swap::<1, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP2 => stack::swap::<2, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP3 => stack::swap::<3, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP4 => stack::swap::<4, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP5 => stack::swap::<5, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP6 => stack::swap::<6, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP7 => stack::swap::<7, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP8 => stack::swap::<8, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP9 => stack::swap::<9, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP10 => stack::swap::<10, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP11 => stack::swap::<11, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP12 => stack::swap::<12, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP13 => stack::swap::<13, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP14 => stack::swap::<14, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP15 => stack::swap::<15, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAP16 => stack::swap::<16, _, _>(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- EOF stack ops (0xE6-0xE8) ---
            DUPN => stack::dupn(InstructionContext {
                interpreter: interp,
                host,
            }),
            SWAPN => stack::swapn(InstructionContext {
                interpreter: interp,
                host,
            }),
            EXCHANGE => stack::exchange(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Log (0xA0-0xA4) ---
            LOG0 => host_instr::log::<0, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            LOG1 => host_instr::log::<1, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            LOG2 => host_instr::log::<2, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            LOG3 => host_instr::log::<3, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            LOG4 => host_instr::log::<4, _>(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Contract (0xF0-0xFF) ---
            CREATE => contract::create::<_, false, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            CALL => contract::call(InstructionContext {
                interpreter: interp,
                host,
            }),
            CALLCODE => contract::call_code(InstructionContext {
                interpreter: interp,
                host,
            }),
            RETURN => control::ret(InstructionContext {
                interpreter: interp,
                host,
            }),
            DELEGATECALL => contract::delegate_call(InstructionContext {
                interpreter: interp,
                host,
            }),
            CREATE2 => contract::create::<_, true, _>(InstructionContext {
                interpreter: interp,
                host,
            }),
            STATICCALL => contract::static_call(InstructionContext {
                interpreter: interp,
                host,
            }),
            REVERT => control::revert(InstructionContext {
                interpreter: interp,
                host,
            }),
            INVALID => control::invalid(InstructionContext {
                interpreter: interp,
                host,
            }),
            SELFDESTRUCT => host_instr::selfdestruct(InstructionContext {
                interpreter: interp,
                host,
            }),

            // --- Unknown/invalid opcodes ---
            _ => control::unknown(InstructionContext {
                interpreter: interp,
                host,
            }),
        }
    }

    interp.take_next_action()
}
