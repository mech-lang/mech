use mech_core::{
    BytecodeCompilerContext, MResult, Register, ValueCell, compile_absent_register,
    compile_value_cell_register,
};

use crate::CompileCtx;

/// Short-lived source-planning ownership retained until artifact compilation.
///
/// Source values are always explicit canonical cells. Semantic wrappers are
/// encoded in the cell schema rather than reconstructed as compiler-only
/// legacy values.
#[derive(Clone, Debug)]
pub(super) enum CompilerValueSource {
    Cell(ValueCell),
    Absent,
}

impl CompilerValueSource {
    pub(super) fn compile_register(
        &self,
        context: &mut dyn BytecodeCompilerContext,
    ) -> MResult<Register> {
        match self {
            Self::Cell(cell) => compile_value_cell_register(cell, context),
            Self::Absent => compile_absent_register(context),
        }
    }

    pub(super) fn retain_cells(&self, context: &mut CompileCtx) -> MResult<()> {
        match self {
            Self::Cell(cell) => context.retain_compiler_value_cell(cell),
            Self::Absent => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompileCtx;

    #[test]
    fn compiler_sources_retain_canonical_cell_identity() {
        let cell = ValueCell::from_exact(7.0_f64).unwrap();
        let source = CompilerValueSource::Cell(cell.clone());
        let mut context = CompileCtx::new();

        let direct = compile_value_cell_register(&cell, &mut context).unwrap();
        let first = source.compile_register(&mut context).unwrap();
        let second = source.compile_register(&mut context).unwrap();

        assert_eq!(direct, first);
        assert_eq!(first, second);
    }

    #[test]
    fn source_absence_compiles_without_becoming_canonical_unit() {
        let source = CompilerValueSource::Absent;
        let mut context = CompileCtx::new();

        source.retain_cells(&mut context).unwrap();
        source.compile_register(&mut context).unwrap();
    }
}
