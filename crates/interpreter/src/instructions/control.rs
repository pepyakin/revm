use crate::{
    interpreter::Interpreter,
    interpreter_types::{InterpreterTypes, Jumps, LoopControl, MemoryTr, RuntimeFlag, StackTr},
    InstructionResult, InterpreterAction,
};
use context_interface::{cfg::GasParams, Host};
use primitives::{Bytes, U256};

use crate::InstructionContext;

/// Implements the JUMP instruction.
///
/// Unconditional jump to a valid destination.
pub fn jump<ITy: InterpreterTypes, H: ?Sized>(context: InstructionContext<'_, H, ITy>) {
    let Some(target) = context.interpreter.stack.pop() else {
        context.interpreter.halt_underflow();
        return;
    };
    jump_inner(context.interpreter, target);
}

/// Implements the JUMPI instruction.
///
/// Conditional jump to a valid destination if condition is true.
pub fn jumpi<WIRE: InterpreterTypes, H: ?Sized>(context: InstructionContext<'_, H, WIRE>) {
    let cond_is_zero = {
        let stack = &context.interpreter.stack;
        let len = stack.len();
        if len < 2 {
            context.interpreter.halt_underflow();
            return;
        }
        stack.data()[len - 2].is_zero()
    };

    if cond_is_zero {
        let _ = context.interpreter.stack.discard::<2>();
        return;
    }

    let Some(target) = context.interpreter.stack.pop() else {
        context.interpreter.halt_underflow();
        return;
    };
    let _ = context.interpreter.stack.discard::<1>();
    jump_inner(context.interpreter, target);
}

/// Internal helper function for jump operations.
///
/// Validates jump target and performs the actual jump.
#[inline(always)]
fn jump_inner<WIRE: InterpreterTypes>(interpreter: &mut Interpreter<WIRE>, target: U256) {
    let target = match target.as_limbs() {
        x if (x[0] > usize::MAX as u64) | (x[1] != 0) | (x[2] != 0) | (x[3] != 0) => {
            interpreter.halt(InstructionResult::InvalidJump);
            return;
        }
        x => x[0] as usize,
    };
    if !interpreter.bytecode.is_valid_legacy_jump(target) {
        interpreter.halt(InstructionResult::InvalidJump);
        return;
    }
    // SAFETY: `is_valid_jump` ensures that `dest` is in bounds.
    interpreter.bytecode.absolute_jump(target);
}

/// Implements the JUMPDEST instruction.
///
/// Marks a valid destination for jump operations.
pub fn jumpdest<WIRE: InterpreterTypes, H: ?Sized>(_context: InstructionContext<'_, H, WIRE>) {}

/// Implements the PC instruction.
///
/// Pushes the current program counter onto the stack.
pub fn pc<WIRE: InterpreterTypes, H: ?Sized>(context: InstructionContext<'_, H, WIRE>) {
    // - 1 because we have already advanced the instruction pointer in `Interpreter::step`
    push!(
        context.interpreter,
        U256::from(context.interpreter.bytecode.pc() - 1)
    );
}

#[inline]
/// Internal helper function for return operations.
///
/// Handles memory data retrieval and sets the return action.
fn return_inner(
    interpreter: &mut Interpreter<impl InterpreterTypes>,
    gas_params: &GasParams,
    instruction_result: InstructionResult,
) {
    popn!([offset, len], interpreter);
    let len = as_usize_or_fail!(interpreter, len);
    // Important: Offset must be ignored if len is zeros
    let mut output = Bytes::default();
    if len != 0 {
        let offset = as_usize_or_fail!(interpreter, offset);
        if !interpreter.resize_memory(gas_params, offset, len) {
            return;
        }
        output = interpreter.memory.slice_len(offset, len).to_vec().into()
    }

    interpreter
        .bytecode
        .set_action(InterpreterAction::new_return(
            instruction_result,
            output,
            interpreter.gas,
        ));
}

/// Implements the RETURN instruction.
///
/// Halts execution and returns data from memory.
pub fn ret<WIRE: InterpreterTypes, H: Host + ?Sized>(context: InstructionContext<'_, H, WIRE>) {
    return_inner(
        context.interpreter,
        context.host.gas_params(),
        InstructionResult::Return,
    );
}

/// EIP-140: REVERT instruction
pub fn revert<WIRE: InterpreterTypes, H: Host + ?Sized>(context: InstructionContext<'_, H, WIRE>) {
    check!(context.interpreter, BYZANTIUM);
    return_inner(
        context.interpreter,
        context.host.gas_params(),
        InstructionResult::Revert,
    );
}

/// Stop opcode. This opcode halts the execution.
pub fn stop<WIRE: InterpreterTypes, H: ?Sized>(context: InstructionContext<'_, H, WIRE>) {
    context.interpreter.halt(InstructionResult::Stop);
}

/// Invalid opcode. This opcode halts the execution.
pub fn invalid<WIRE: InterpreterTypes, H: ?Sized>(context: InstructionContext<'_, H, WIRE>) {
    context.interpreter.halt(InstructionResult::InvalidFEOpcode);
}

/// Unknown opcode. This opcode halts the execution.
pub fn unknown<WIRE: InterpreterTypes, H: ?Sized>(context: InstructionContext<'_, H, WIRE>) {
    context.interpreter.halt(InstructionResult::OpcodeNotFound);
}

#[cfg(test)]
mod tests {
    use crate::{
        host::DummyHost,
        instructions::instruction_table,
        interpreter::{EthInterpreter, ExtBytecode, InputsImpl, SharedMemory},
        Interpreter,
    };
    use bytecode::opcode::*;
    use bytecode::Bytecode;
    use primitives::{hardfork::SpecId, Bytes, U256};

    fn run_bytecode(code: &[u8]) -> Interpreter {
        let bytecode = Bytecode::new_raw(Bytes::copy_from_slice(code));
        let mut interpreter = Interpreter::<EthInterpreter>::new(
            SharedMemory::new(),
            ExtBytecode::new(bytecode),
            InputsImpl::default(),
            false,
            SpecId::AMSTERDAM,
            u64::MAX,
        );
        let table = instruction_table::<EthInterpreter, DummyHost>();
        let mut host = DummyHost::new(SpecId::AMSTERDAM);
        interpreter.run_plain(&table, &mut host);
        interpreter
    }

    #[test]
    fn jumpi_false_skips_invalid_target_validation() {
        let interpreter = run_bytecode(&[PUSH1, 0x00, PUSH1, 0xff, JUMPI, PUSH1, 0x2a]);

        assert_eq!(interpreter.stack.data(), &[U256::from(0x2a)]);
    }
}
