//! General native byte permutations and checked table translation. No alphabet,
//! record format, line width or benchmark name is built into these loops.
use cranelift_codegen::ir::{
    AbiParam, ConstantData, InstBuilder, MemFlags, Value, condcodes::IntCC, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
use mech_engine::native_sequence::{Constant, Program};

pub(super) fn table(from: &[u8], to: &[u8], missing: &[u8]) -> Result<[u16; 256], String> {
    if from.len() != to.len() {
        return Err("byte table lengths differ".into());
    }
    let mut result = match missing {
        b"error" => [u16::MAX; 256],
        b"identity" => std::array::from_fn(|i| i as u16),
        _ => return Err("byte table missing policy must be error or identity".into()),
    };
    let mut seen = [false; 256];
    for (&key, &value) in from.iter().zip(to) {
        if seen[key as usize] && result[key as usize] != value as u16 {
            return Err("conflicting byte table entry".into());
        }
        result[key as usize] = value as u16;
        seen[key as usize] = true;
    }
    Ok(result)
}
fn constant_table(graph: &Program, id: usize) -> Option<[u16; 256]> {
    let node = graph.nodes.get(id.checked_sub(graph.constants.len())?)?;
    if node.op != 48 {
        return None;
    }
    let args: Option<Vec<_>> = node
        .args
        .iter()
        .map(|&id| match graph.constants.get(id) {
            Some(Constant::Text(s)) => Some(s.as_bytes()),
            _ => None,
        })
        .collect();
    let args = args?;
    table(args[0], args[1], args[2]).ok()
}
// Prove a general lookup can be represented by two vector table shuffles.
// ASCII case folding is eligible only when every folded pair is identical.
fn compressed(table: &[u16; 256]) -> Option<(u8, u8, Vec<u8>)> {
    let mask = if (0..256).all(|i| table[i] == table[i & 0xdf]) {
        0xdf
    } else {
        0xff
    };
    let keys: Vec<_> = (0..256)
        .filter(|&i| i & mask as usize == i && table[i] != u16::MAX)
        .collect();
    let lo = *keys.first()?;
    let hi = *keys.last()?;
    if hi - lo >= 32 || keys.iter().any(|&i| table[i] == 0) {
        return None;
    }
    let bytes = (0..32)
        .map(|i| {
            let k = lo + i;
            if k < 256 && table[k] != u16::MAX {
                table[k] as u8
            } else {
                0
            }
        })
        .collect();
    Some((mask, lo as u8, bytes))
}
fn vector(b: &mut FunctionBuilder, bytes: Vec<u8>) -> Value {
    let c = b.func.dfg.constants.insert(ConstantData::from(bytes));
    b.ins().vconst(types::I8X16, c)
}
fn translate(
    b: &mut FunctionBuilder,
    byte: Value,
    lut: Value,
    error: cranelift_codegen::ir::Block,
) -> Value {
    let key = b.ins().uextend(types::I64, byte);
    let offset = b.ins().ishl_imm(key, 1);
    let addr = b.ins().iadd(lut, offset);
    let value = b.ins().load(types::I16, MemFlags::new(), addr, 0);
    let bad = b.ins().icmp_imm(IntCC::Equal, value, 65535);
    let next = b.create_block();
    b.ins().brif(bad, error, &[], next, &[]);
    b.switch_to_block(next);
    b.ins().ireduce(types::I8, value)
}
fn translate_vector(
    b: &mut FunctionBuilder,
    byte: Value,
    compact: &(u8, u8, Vec<u8>),
    error: cranelift_codegen::ir::Block,
) -> Value {
    let mask = vector(b, vec![compact.0; 16]);
    let base = vector(b, vec![compact.1; 16]);
    let index = b.ins().band(byte, mask);
    let index = b.ins().isub(index, base);
    let t0 = vector(b, compact.2[..16].to_vec());
    let t1 = vector(b, compact.2[16..].to_vec());
    let a = b.ins().swizzle(t0, index);
    let sixteen = vector(b, vec![16; 16]);
    let index = b.ins().isub(index, sixteen);
    let c = b.ins().swizzle(t1, index);
    let value = b.ins().bor(a, c);
    let z = vector(b, vec![0; 16]);
    let bad = b.ins().icmp(IntCC::Equal, value, z);
    let bad = b.ins().vany_true(bad);
    let next = b.create_block();
    b.ins().brif(bad, error, &[], next, &[]);
    b.switch_to_block(next);
    value
}
pub(super) fn emit<M: Module>(
    module: &mut M,
    graph: &Program,
    args: &[usize],
    op: u64,
    name: &str,
    simd: bool,
) -> Result<FuncId, String> {
    let reverse = matches!(op, 44 | 46);
    let mapping = op != 44;
    let compact = if simd && mapping {
        constant_table(graph, args[1]).and_then(|t| compressed(&t))
    } else {
        None
    };
    let vectorized = simd && (!mapping || compact.is_some());
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 6];
    ctx.func.signature.returns.push(AbiParam::new(types::I32));
    let id = module
        .declare_function(name, Linkage::Local, &ctx.func.signature)
        .map_err(|e| e.to_string())?;
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
    let entry = b.create_block();
    let check = b.create_block();
    let scalar = b.create_block();
    let wide = b.create_block();
    let exit = b.create_block();
    let error = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(check, types::I64);
    b.switch_to_block(entry);
    let p = b.block_params(entry).to_vec();
    b.ins().jump(check, &[p[4].into()]);
    b.switch_to_block(check);
    let i = b.block_params(check)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, i, p[5]);
    let test = b.create_block();
    b.ins().brif(more, test, &[], exit, &[]);
    b.switch_to_block(test);
    if vectorized {
        let end = b.ins().iadd_imm(i, 16);
        let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, p[5]);
        let fits = if reverse {
            let two = b.ins().iadd(end, end);
            let middle = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, two, p[2]);
            b.ins().band(fits, middle)
        } else {
            fits
        };
        b.ins().brif(fits, wide, &[], scalar, &[]);
    } else {
        b.ins().jump(scalar, &[]);
    }
    b.switch_to_block(scalar);
    let j = if reverse {
        let last = b.ins().iadd_imm(p[2], -1);
        b.ins().isub(last, i)
    } else {
        i
    };
    let addr = b.ins().iadd(p[0], j);
    let right = b.ins().load(types::I8, MemFlags::new(), addr, 0);
    let left = if reverse {
        let addr = b.ins().iadd(p[0], i);
        Some(b.ins().load(types::I8, MemFlags::new(), addr, 0))
    } else {
        None
    };
    let right = if mapping {
        translate(&mut b, right, p[3], error)
    } else {
        right
    };
    let left = left.map(|x| {
        if mapping {
            translate(&mut b, x, p[3], error)
        } else {
            x
        }
    });
    let addr = b.ins().iadd(p[1], i);
    b.ins().store(MemFlags::new(), right, addr, 0);
    if let Some(left) = left {
        let addr = b.ins().iadd(p[1], j);
        b.ins().store(MemFlags::new(), left, addr, 0);
    }
    let next = b.ins().iadd_imm(i, 1);
    b.ins().jump(check, &[next.into()]);
    b.switch_to_block(wide);
    if vectorized {
        let j = if reverse {
            let end = b.ins().iadd_imm(i, 16);
            b.ins().isub(p[2], end)
        } else {
            i
        };
        let addr = b.ins().iadd(p[0], j);
        let right = b.ins().load(types::I8X16, MemFlags::new(), addr, 0);
        let left = if reverse {
            let addr = b.ins().iadd(p[0], i);
            Some(b.ins().load(types::I8X16, MemFlags::new(), addr, 0))
        } else {
            None
        };
        let rev = |b: &mut FunctionBuilder, x| {
            let mask = b
                .func
                .dfg
                .immediates
                .push(ConstantData::from((0..16u8).rev().collect::<Vec<_>>()));
            b.ins().shuffle(x, x, mask)
        };
        let right = if reverse { rev(&mut b, right) } else { right };
        let left = left.map(|x| rev(&mut b, x));
        let right = if let Some(c) = &compact {
            translate_vector(&mut b, right, c, error)
        } else {
            right
        };
        let left = left.map(|x| {
            if let Some(c) = &compact {
                translate_vector(&mut b, x, c, error)
            } else {
                x
            }
        });
        let addr = b.ins().iadd(p[1], i);
        b.ins().store(MemFlags::new(), right, addr, 0);
        if let Some(left) = left {
            let addr = b.ins().iadd(p[1], j);
            b.ins().store(MemFlags::new(), left, addr, 0);
        }
        let next = b.ins().iadd_imm(i, 16);
        b.ins().jump(check, &[next.into()]);
    } else {
        b.ins().jump(exit, &[]);
    }
    b.switch_to_block(exit);
    let z = b.ins().iconst(types::I32, 0);
    b.ins().return_(&[z]);
    b.switch_to_block(error);
    let one = b.ins().iconst(types::I32, 1);
    b.ins().return_(&[one]);
    b.seal_all_blocks();
    b.finalize();
    module
        .define_function(id, &mut ctx)
        .map_err(|e| e.to_string())?;
    Ok(id)
}
pub(super) fn emit_records<M: Module>(module: &mut M, name: &str) -> Result<FuncId, String> {
    let mut ctx = module.make_context();
    ctx.func.signature.params.push(AbiParam::new(types::I64));
    let id = module
        .declare_function(name, Linkage::Local, &ctx.func.signature)
        .map_err(|e| e.to_string())?;
    let mut sig = module.make_signature();
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let step = module
        .declare_function("mech_sequence_record_step", Linkage::Import, &sig)
        .map_err(|e| e.to_string())?;
    let step = module.declare_func_in_func(step, &mut ctx.func);
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
    let entry = b.create_block();
    let body = b.create_block();
    let exit = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let state = b.block_params(entry)[0];
    b.ins().jump(body, &[]);
    b.switch_to_block(body);
    let call = b.ins().call(step, &[state]);
    let status = b.inst_results(call)[0];
    let more = b.ins().icmp_imm(IntCC::Equal, status, 0);
    b.ins().brif(more, body, &[], exit, &[]);
    b.switch_to_block(exit);
    b.ins().return_(&[]);
    b.seal_all_blocks();
    b.finalize();
    module
        .define_function(id, &mut ctx)
        .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Compile constant-width serialization as vector copies into cache-sized
/// output spans. Eligibility is source constants, not an input line pattern.
pub(super) fn emit_wrap<M: Module>(
    module: &mut M,
    graph: &Program,
    args: &[usize],
    name: &str,
) -> Result<Option<FuncId>, String> {
    let Some(Constant::Integer(width)) = graph.constants.get(args[1]) else {
        return Ok(None);
    };
    let Some(Constant::Text(separator)) = graph.constants.get(args[2]) else {
        return Ok(None);
    };
    if !(1..=256).contains(width) || separator.len() > 8 {
        return Ok(None);
    }
    let width = *width as usize;
    let separator = separator.as_bytes();
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    let id = module
        .declare_function(name, Linkage::Local, &ctx.func.signature)
        .map_err(|e| e.to_string())?;
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
    let entry = b.create_block();
    let check = b.create_block();
    let full = b.create_block();
    let tail_check = b.create_block();
    let tail = b.create_block();
    let suffix = b.create_block();
    let exit = b.create_block();
    b.append_block_params_for_function_params(entry);
    for block in [check, tail_check, suffix] {
        b.append_block_param(block, types::I64);
        b.append_block_param(block, types::I64);
    }
    b.switch_to_block(entry);
    let p = b.block_params(entry).to_vec();
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(check, &[zero.into(), zero.into()]);
    b.switch_to_block(check);
    let i = b.block_params(check)[0];
    let o = b.block_params(check)[1];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, i, p[2]);
    let fits = b.create_block();
    b.ins().brif(more, fits, &[], exit, &[]);
    b.switch_to_block(fits);
    let end = b.ins().iadd_imm(i, width as i64);
    let enough = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, p[2]);
    b.ins()
        .brif(enough, full, &[], tail_check, &[i.into(), o.into()]);
    b.switch_to_block(full);
    let src = b.ins().iadd(p[0], i);
    let dst = b.ins().iadd(p[1], o);
    let mut offset = 0;
    for (size, ty) in [
        (16, types::I8X16),
        (8, types::I64),
        (4, types::I32),
        (2, types::I16),
        (1, types::I8),
    ] {
        while offset + size <= width {
            let value = b.ins().load(ty, MemFlags::new(), src, offset as i32);
            b.ins().store(MemFlags::new(), value, dst, offset as i32);
            offset += size;
        }
    }
    let next = b.ins().iadd_imm(i, width as i64);
    let output = b.ins().iadd_imm(o, width as i64);
    b.ins().jump(suffix, &[next.into(), output.into()]);
    b.switch_to_block(tail_check);
    let i = b.block_params(tail_check)[0];
    let o = b.block_params(tail_check)[1];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, i, p[2]);
    b.ins().brif(more, tail, &[], suffix, &[i.into(), o.into()]);
    b.switch_to_block(tail);
    let src = b.ins().iadd(p[0], i);
    let dst = b.ins().iadd(p[1], o);
    let byte = b.ins().load(types::I8, MemFlags::new(), src, 0);
    b.ins().store(MemFlags::new(), byte, dst, 0);
    let i = b.ins().iadd_imm(i, 1);
    let o = b.ins().iadd_imm(o, 1);
    b.ins().jump(tail_check, &[i.into(), o.into()]);
    b.switch_to_block(suffix);
    let i = b.block_params(suffix)[0];
    let o = b.block_params(suffix)[1];
    let dst = b.ins().iadd(p[1], o);
    for (offset, &byte) in separator.iter().enumerate() {
        let byte = b.ins().iconst(types::I8, byte as i64);
        b.ins().store(MemFlags::new(), byte, dst, offset as i32);
    }
    let o = b.ins().iadd_imm(o, separator.len() as i64);
    b.ins().jump(check, &[i.into(), o.into()]);
    b.switch_to_block(exit);
    b.ins().return_(&[]);
    b.seal_all_blocks();
    b.finalize();
    module
        .define_function(id, &mut ctx)
        .map_err(|e| e.to_string())?;
    Ok(Some(id))
}
