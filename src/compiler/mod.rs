use std::fmt;

use mech_core::{DecodedInstr, ParsedProgram, Program};
use mech_interpreter::Interpreter;
use mech_syntax::parser;

pub mod simd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    None,
    Less,
    Default,
    Aggressive,
}

impl fmt::Display for OptLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptLevel::None => write!(f, "O0"),
            OptLevel::Less => write!(f, "O1"),
            OptLevel::Default => write!(f, "O2"),
            OptLevel::Aggressive => write!(f, "O3"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompilerConfig {
    pub module_name: String,
    pub target_triple: String,
    pub opt_level: OptLevel,
    pub enable_simd_matrices: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            module_name: "mech_module".to_string(),
            target_triple: std::env::consts::ARCH.to_string(),
            opt_level: OptLevel::Default,
            enable_simd_matrices: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub bytecode: Vec<u8>,
    pub parsed_program: ParsedProgram,
    pub llvm_ir: String,
    pub notes: Vec<String>,
}

#[derive(Debug)]
pub enum CompilerError {
    EmptyInput,
    Parse(String),
    Lowering(String),
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerError::EmptyInput => write!(f, "compiler input cannot be empty"),
            CompilerError::Parse(msg) => write!(f, "parse failed: {msg}"),
            CompilerError::Lowering(msg) => write!(f, "bytecode lowering failed: {msg}"),
        }
    }
}

impl std::error::Error for CompilerError {}

#[derive(Debug, Clone)]
pub struct LlvmCompiler {
    config: CompilerConfig,
}

impl LlvmCompiler {
    pub fn new(config: CompilerConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &CompilerConfig {
        &self.config
    }

    /// Parse Mech source and compile through the existing Mech bytecode pipeline,
    /// then lower that bytecode into LLVM IR text.
    pub fn compile_source(&self, source: &str) -> Result<CompiledModule, CompilerError> {
        if source.trim().is_empty() {
            return Err(CompilerError::EmptyInput);
        }

        let program = parser::parse(source).map_err(|e| CompilerError::Parse(format!("{e:?}")))?;
        self.compile_program(&program)
    }

    /// Compile an already-parsed Mech AST (`Program`) to bytecode and LLVM IR.
    pub fn compile_program(&self, program: &Program) -> Result<CompiledModule, CompilerError> {
        let mut interpreter = Interpreter::new(0);
        interpreter
            .interpret(program)
            .map_err(|e| CompilerError::Lowering(format!("{e:?}")))?;

        let bytecode = interpreter
            .compile()
            .map_err(|e| CompilerError::Lowering(format!("{e:?}")))?;

        let parsed_program = ParsedProgram::from_bytes(&bytecode)
            .map_err(|e| CompilerError::Lowering(format!("{e:?}")))?;

        let mut notes = vec![
            format!("optimization level: {}", self.config.opt_level),
            format!("target: {}", self.config.target_triple),
            format!("instructions: {}", parsed_program.instrs.len()),
            format!("registers: {}", parsed_program.header.reg_count),
        ];

        let has_matrix_feature = parsed_program.features.iter().any(|f| {
            let matrix_range = 23u16..=37u16;
            matrix_range.contains(&((*f & 0xFFFF) as u16))
        });

        if self.config.enable_simd_matrices && has_matrix_feature {
            notes.push("SIMD matrix kernels are enabled for matrix bytecode ops".to_string());
        }

        let llvm_ir = self.lower_bytecode_to_llvm_ir(&parsed_program);

        Ok(CompiledModule {
            bytecode,
            parsed_program,
            llvm_ir,
            notes,
        })
    }

    fn lower_bytecode_to_llvm_ir(&self, program: &ParsedProgram) -> String {
        let mut ir = String::new();
        ir.push_str(&format!("; ModuleID = '{}'\n", self.config.module_name));
        ir.push_str("source_filename = \"mech-bytecode\"\n");
        ir.push_str(&format!(
            "target triple = \"{}\"\n\n",
            self.config.target_triple
        ));
        ir.push_str("declare void @mech_constload(i32, i32)\n");
        ir.push_str("declare void @mech_nullop(i64, i32)\n");
        ir.push_str("declare void @mech_unop(i64, i32, i32)\n");
        ir.push_str("declare void @mech_binop(i64, i32, i32, i32)\n");
        ir.push_str("declare void @mech_ternop(i64, i32, i32, i32, i32)\n");
        ir.push_str("declare void @mech_quadop(i64, i32, i32, i32, i32, i32)\n");
        ir.push_str("declare void @mech_vararg(i64, i32, ptr, i32)\n\n");

        ir.push_str("define i32 @mech_main() {\nentry:\n");
        for (idx, ins) in program.instrs.iter().enumerate() {
            ir.push_str(&format!("  ; bc[{idx}] = {:?}\n", ins));
            match ins {
                DecodedInstr::ConstLoad { dst, const_id } => {
                    ir.push_str(&format!(
                        "  call void @mech_constload(i32 {dst}, i32 {const_id})\n"
                    ));
                }
                DecodedInstr::NullOp { fxn_id, dst } => {
                    ir.push_str(&format!(
                        "  call void @mech_nullop(i64 {fxn_id}, i32 {dst})\n"
                    ));
                }
                DecodedInstr::UnOp { fxn_id, dst, src } => {
                    ir.push_str(&format!(
                        "  call void @mech_unop(i64 {fxn_id}, i32 {dst}, i32 {src})\n"
                    ));
                }
                DecodedInstr::BinOp {
                    fxn_id,
                    dst,
                    lhs,
                    rhs,
                } => {
                    ir.push_str(&format!(
                        "  call void @mech_binop(i64 {fxn_id}, i32 {dst}, i32 {lhs}, i32 {rhs})\n"
                    ));
                    if self.config.enable_simd_matrices {
                        ir.push_str(
                            "  ; SIMD candidate: matrix binop can lower to vector kernel\n",
                        );
                    }
                }
                DecodedInstr::TernOp {
                    fxn_id,
                    dst,
                    a,
                    b,
                    c,
                } => {
                    ir.push_str(&format!(
                        "  call void @mech_ternop(i64 {fxn_id}, i32 {dst}, i32 {a}, i32 {b}, i32 {c})\n"
                    ));
                }
                DecodedInstr::QuadOp {
                    fxn_id,
                    dst,
                    a,
                    b,
                    c,
                    d,
                } => {
                    ir.push_str(&format!(
                        "  call void @mech_quadop(i64 {fxn_id}, i32 {dst}, i32 {a}, i32 {b}, i32 {c}, i32 {d})\n"
                    ));
                }
                DecodedInstr::VarArg { fxn_id, dst, args } => {
                    ir.push_str(&format!(
                        "  ; vararg register list length={}\n  call void @mech_vararg(i64 {fxn_id}, i32 {dst}, ptr null, i32 {})\n",
                        args.len(),
                        args.len()
                    ));
                }
                DecodedInstr::Ret { src } => {
                    ir.push_str(&format!("  ; return source register r{src}\n"));
                }
                DecodedInstr::Unknown { opcode, .. } => {
                    ir.push_str(&format!("  ; unknown opcode {opcode}\n"));
                }
            }
        }
        ir.push_str("  ret i32 0\n}\n");
        ir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_source_uses_mech_pipeline() {
        let compiler = LlvmCompiler::new(CompilerConfig::default());
        let output = compiler.compile_source("a := 1\nb := a + 2").unwrap();

        assert!(!output.bytecode.is_empty());
        assert!(!output.parsed_program.instrs.is_empty());
        assert!(output.llvm_ir.contains("@mech_binop") || output.llvm_ir.contains("@mech_unop"));
    }
}
