//! Diagnostic inventory for finding repeated native regions after CFG cleanup.
use super::checked_traversals::Candidate;
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{Block, Function, InstructionData, Opcode, Value, ValueDef};
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
};

pub(super) fn families(
    function: &Function,
    windows: &str,
    source_sites: usize,
) -> Result<Vec<Vec<Candidate>>, NativeCompileError> {
    windows
        .split(';')
        .map(|list| candidates(function, list, source_sites))
        .collect()
}

fn candidates(
    function: &Function,
    windows: &str,
    source_sites: usize,
) -> Result<Vec<Candidate>, NativeCompileError> {
    windows
        .split(',')
        .filter(|window| !window.is_empty())
        .map(|window| {
            let fields = window
                .split(':')
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(backend)?;
            let [entry, continuation, origin_base] = fields.as_slice() else {
                return Err(backend("expected layout entry:continuation:source base"));
            };
            if entry == continuation
                || [entry, continuation]
                    .iter()
                    .any(|&&id| id as usize >= function.dfg.num_blocks())
                || *origin_base as usize >= source_sites
            {
                return Err(backend(
                    "layout controller boundary is outside the current function",
                ));
            }
            Ok(Candidate {
                resume: None,
                start: Block::from_u32(*entry),
                exit: Block::from_u32(*continuation),
                header: source_sites + *entry as usize,
                origin_base: Some(*origin_base),
                span: 0,
            })
        })
        .collect()
}

pub(super) fn phase_candidates(
    function: &Function,
    windows: &str,
    first_header: usize,
) -> Result<Vec<Vec<Candidate>>, NativeCompileError> {
    let block = |field: &str| {
        let id = field.parse::<u32>().map_err(backend)?;
        if id as usize >= function.dfg.num_blocks() {
            return Err(backend(
                "checked phase boundary is outside the current function",
            ));
        }
        Ok(Block::from_u32(id))
    };
    windows
        .split(';')
        .map(|family| {
            family
                .split(',')
                .map(|window| {
                    let fields = window.split(':').collect::<Vec<_>>();
                    let [entry, continuation, resume] = fields.as_slice() else {
                        return Err(backend("expected phase entry:continuation:resume"));
                    };
                    let start = block(entry)?;
                    let header = first_header
                        .checked_add(start.as_u32() as usize)
                        .ok_or_else(|| backend("checked phase region identifier overflow"))?;
                    Ok(Candidate {
                        start,
                        exit: block(continuation)?,
                        resume: (*resume != "-").then(|| block(resume)).transpose()?,
                        header,
                        origin_base: Some(0),
                        span: 0,
                    })
                })
                .collect()
        })
        .collect()
}

fn immediate(function: &Function, value: Value) -> Option<i64> {
    let ValueDef::Result(producer, _) = function.dfg.value_def(function.dfg.resolve_aliases(value))
    else {
        return None;
    };
    match function.dfg.insts[producer] {
        InstructionData::UnaryImm {
            opcode: Opcode::Iconst,
            imm,
        } => Some(i64::from(imm)),
        _ => None,
    }
}

pub(super) fn report(function: &Function, path: &Path) -> io::Result<()> {
    let mut out = BufWriter::new(File::create(path)?);
    for block in function.layout.blocks() {
        let Some(last) = function.layout.last_inst(block) else {
            continue;
        };
        let dfg = &function.dfg;
        let instruction = &dfg.insts[last];
        let (status, origin) = if instruction.opcode() == Opcode::Return {
            let args = function.dfg.inst_args(last);
            (
                args.first().and_then(|&v| immediate(function, v)),
                args.get(1).and_then(|&v| immediate(function, v)),
            )
        } else {
            (None, None)
        };
        write!(
            out,
            "{}\t{}\t{}\t{}\t",
            block.as_u32(),
            u8::from(function.layout.is_cold(block)),
            status.map_or(String::new(), |v| v.to_string()),
            origin.map_or(String::new(), |v| v.to_string())
        )?;
        for edge in instruction.branch_destination(&dfg.jump_tables, &dfg.exception_tables) {
            write!(out, "{},", edge.block(&dfg.value_lists).as_u32())?;
        }
        write!(out, "\t")?;
        for &parameter in dfg.block_params(block) {
            write!(out, "{},", dfg.value_type(parameter))?;
        }
        write!(out, "\t")?;
        for inst in function.layout.block_insts(block) {
            write!(out, "{}", dfg.insts[inst].opcode())?;
            for &result in dfg.inst_results(inst) {
                write!(out, ":{}", dfg.value_type(result))?;
            }
            if let InstructionData::Call { func_ref, .. }
            | InstructionData::FuncAddr { func_ref, .. } = dfg.insts[inst]
            {
                // Distinct callees can have identical local instruction shapes.
                // Expose their relocation identity to the region search.
                let name = &dfg.ext_funcs[func_ref].name;
                write!(out, "@{}", name.display(Some(&function.params)))?;
            }
            write!(out, ",")?;
        }
        writeln!(out)?;
    }
    out.flush()
}
