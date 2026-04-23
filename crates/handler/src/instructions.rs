use auto_impl::auto_impl;
use interpreter::{
    instructions::{
        instruction_table_gas_changes_spec, instruction_table_gas_changes_spec_fast,
        InstructionTable,
    },
    Host, Instruction, InterpreterTypes,
};
use primitives::hardfork::SpecId;
use std::boxed::Box;

/// Stores instructions for EVM.
#[auto_impl(&mut, Box)]
pub trait InstructionProvider {
    /// Context type.
    type Context;
    /// Interpreter types.
    type InterpreterTypes: InterpreterTypes;

    /// Returns the instruction table that is used by EvmTr to execute instructions.
    ///
    /// This table preserves per-opcode dispatch (including JUMPDEST) and is what
    /// inspectors observe during tracing.
    fn instruction_table(&self) -> &InstructionTable<Self::InterpreterTypes, Self::Context>;

    /// Returns the instruction table used by the non-inspector hot path.
    ///
    /// Implementations may return a variant with optimizations that change the
    /// per-step dispatch profile (e.g. folding JUMPDEST into JUMP/JUMPI). Defaults
    /// to [`Self::instruction_table`] so existing providers remain correct.
    fn instruction_table_fast(&self) -> &InstructionTable<Self::InterpreterTypes, Self::Context> {
        self.instruction_table()
    }
}

/// Ethereum instruction contains list of mainnet instructions that is used for Interpreter execution.
#[derive(Debug)]
pub struct EthInstructions<WIRE: InterpreterTypes, HOST: ?Sized> {
    /// Table containing instruction implementations indexed by opcode.
    pub instruction_table: Box<InstructionTable<WIRE, HOST>>,
    /// Variant of [`Self::instruction_table`] used by the non-inspector hot path,
    /// with JUMP/JUMPI replaced by JUMPDEST-folding handlers.
    pub instruction_table_fast: Box<InstructionTable<WIRE, HOST>>,
    /// Spec that is used to set gas costs for instructions.
    pub spec: SpecId,
}

impl<WIRE, HOST: Host + ?Sized> Clone for EthInstructions<WIRE, HOST>
where
    WIRE: InterpreterTypes,
{
    fn clone(&self) -> Self {
        Self {
            instruction_table: self.instruction_table.clone(),
            instruction_table_fast: self.instruction_table_fast.clone(),
            spec: self.spec,
        }
    }
}

impl<WIRE, HOST> EthInstructions<WIRE, HOST>
where
    WIRE: InterpreterTypes,
    HOST: Host,
{
    /// Returns `EthInstructions` with mainnet spec.
    #[deprecated(since = "0.2.0", note = "use new_mainnet_with_spec instead")]
    pub fn new_mainnet() -> Self {
        Self::new_mainnet_with_spec(SpecId::default())
    }

    /// Returns `EthInstructions` with mainnet spec.
    pub fn new_mainnet_with_spec(spec: SpecId) -> Self {
        Self {
            instruction_table: Box::new(instruction_table_gas_changes_spec(spec)),
            instruction_table_fast: Box::new(instruction_table_gas_changes_spec_fast(spec)),
            spec,
        }
    }

    /// Returns a new instance of `EthInstructions` with custom instruction table.
    ///
    /// The fast table is a clone of `base_table` — no JUMPDEST-folding is applied
    /// when a fully-custom table is supplied.
    #[inline]
    pub fn new(base_table: InstructionTable<WIRE, HOST>, spec: SpecId) -> Self {
        Self {
            instruction_table: Box::new(base_table),
            instruction_table_fast: Box::new(base_table),
            spec,
        }
    }

    /// Inserts a new instruction into both instruction tables.
    #[inline]
    pub fn insert_instruction(&mut self, opcode: u8, instruction: Instruction<WIRE, HOST>) {
        self.instruction_table[opcode as usize] = instruction;
        self.instruction_table_fast[opcode as usize] = instruction;
    }
}

impl<IT, CTX> InstructionProvider for EthInstructions<IT, CTX>
where
    IT: InterpreterTypes,
    CTX: Host,
{
    type InterpreterTypes = IT;
    type Context = CTX;

    fn instruction_table(&self) -> &InstructionTable<Self::InterpreterTypes, Self::Context> {
        &self.instruction_table
    }

    fn instruction_table_fast(&self) -> &InstructionTable<Self::InterpreterTypes, Self::Context> {
        &self.instruction_table_fast
    }
}
