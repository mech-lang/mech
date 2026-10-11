use cranelift_codegen::{
    ir::{AbiParam, InstBuilder, MemFlags, StackSlotData, StackSlotKind, condcodes::IntCC, types},
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
pub use mech_engine::native_sequence::Constant;
use mech_engine::native_sequence::{Kind, Program as Graph};
use std::collections::BTreeMap;
#[derive(Clone)]
pub struct Program {
    graph: Graph,
}
impl std::ops::Deref for Program {
    type Target = Graph;
    fn deref(&self) -> &Graph {
        &self.graph
    }
}
pub fn compile(source: &str, batch: usize) -> Result<Program, String> {
    Ok(Program {
        graph: mech_engine::native_sequence::compile(
            source,
            batch,
            mech_stdlib::native_sequence_catalog(),
        )?,
    })
}
pub fn compile_parsed(
    source: &mech_engine::native_sequence::ParsedSource,
    batch: usize,
) -> Result<Program, String> {
    Ok(Program {
        graph: mech_engine::native_sequence::compile_parsed(
            source,
            batch,
            mech_stdlib::native_sequence_catalog(),
        )?,
    })
}
impl Program {
    pub fn scheduled(self) -> Result<Self, String> {
        Ok(Self {
            graph: self.graph.scheduled()?,
        })
    }
    pub fn fuse_record_encoding(self) -> Self {
        Self {
            graph: self.graph.fuse_record_encoding(),
        }
    }
    pub fn object(&self, batch: usize) -> Result<Vec<u8>, String> {
        self.object_with_pack8(batch, false)
    }

    /// Pack eight two-bit symbols with wide integer operations when enabled.
    /// Other alphabets and every boundary/tail use the scalar checked kernel.
    pub fn object_with_pack8(&self, batch: usize, pack8: bool) -> Result<Vec<u8>, String> {
        self.object_with_packing(batch, pack8, false)
    }

    /// Optional two-lane integer SIMD for the eight-symbol packing path.
    pub fn object_with_packing(
        &self,
        batch: usize,
        pack8: bool,
        simd: bool,
    ) -> Result<Vec<u8>, String> {
        self.object_with_array_simd(batch, pack8, simd, simd)
    }
    pub fn object_with_array_simd(
        &self,
        batch: usize,
        pack8: bool,
        simd: bool,
        array_simd: bool,
    ) -> Result<Vec<u8>, String> {
        if simd && !pack8 {
            return Err("SIMD packing requires pack8".into());
        }
        if !(1..=8192).contains(&batch) {
            return Err("batch must be 1..8192".into());
        }
        let mut flags = settings::builder();
        flags.set("is_pic", "true").map_err(|e| e.to_string())?;
        flags.set("opt_level", "speed").map_err(|e| e.to_string())?;
        let isa = cranelift_native::builder()
            .map_err(|e| e.to_string())?
            .finish(settings::Flags::new(flags))
            .map_err(|e| e.to_string())?;
        let mut module = ObjectModule::new(
            ObjectBuilder::new(isa, "mech_sequence", default_libcall_names())
                .map_err(|e| e.to_string())?,
        );
        self.define(&mut module, batch, pack8, simd, array_simd)?;
        module.finish().emit().map_err(|e| e.to_string())
    }

    pub(super) fn define<M: Module>(
        &self,
        module: &mut M,
        batch: usize,
        pack8: bool,
        simd: bool,
        array_simd: bool,
    ) -> Result<cranelift_module::FuncId, String> {
        let ptr = module.target_config().pointer_type();
        if ptr != types::I64 {
            return Err("collection ABI currently requires 64-bit host".into());
        }
        let mut dispatch_sig = module.make_signature();
        dispatch_sig.params = vec![AbiParam::new(ptr); 8];
        dispatch_sig.returns.push(AbiParam::new(ptr));
        let dispatch = module
            .declare_function("mech_sequence_dispatch", Linkage::Import, &dispatch_sig)
            .map_err(|e| e.to_string())?;
        let mut counter_sig = module.make_signature();
        counter_sig.params = vec![AbiParam::new(ptr); 3];
        let counter = module
            .declare_function("mech_sequence_increment", Linkage::Import, &counter_sig)
            .map_err(|e| e.to_string())?;
        fn collect<'a>(
            graph: &'a Graph,
            output: Option<usize>,
            graphs: &mut Vec<(&'a Graph, Option<usize>)>,
            children: &mut BTreeMap<(usize, usize), usize>,
        ) -> usize {
            let index = graphs.len();
            graphs.push((graph, output));
            for (constant, c) in graph.constants.iter().enumerate() {
                if let Constant::Function(f) = c {
                    let child = collect(&f.program, Some(f.output), graphs, children);
                    children.insert((index, constant), child);
                }
            }
            index
        }
        let mut graphs = Vec::new();
        let mut children = BTreeMap::new();
        collect(&self.graph, None, &mut graphs, &mut children);
        let mut entry_sig = module.make_signature();
        entry_sig.params.push(AbiParam::new(ptr));
        let entries: Vec<_> = (0..graphs.len())
            .map(|i| {
                module
                    .declare_function(
                        &if i == 0 {
                            "mech_sequence_entry".into()
                        } else {
                            format!("mech_callable_{i}")
                        },
                        if i == 0 {
                            Linkage::Export
                        } else {
                            Linkage::Local
                        },
                        &entry_sig,
                    )
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<_, _>>()?;
        let mut kernels = BTreeMap::new();
        for (bits, width) in graphs
            .iter()
            .flat_map(|(g, _)| g.nodes.iter())
            .filter_map(|n| n.kernel)
        {
            if kernels.contains_key(&(bits, width)) {
                continue;
            }
            let name = format!("mech_window_{bits}_{width}");
            let mut ctx = module.make_context();
            ctx.func.signature = counter_sig.clone();
            let id = module
                .declare_function(&name, Linkage::Local, &ctx.func.signature)
                .map_err(|e| e.to_string())?;
            let counter_ref = module.declare_func_in_func(counter, &mut ctx.func);
            let mut fbctx = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let entry = b.create_block();
            let loop_block = b.create_block();
            let scalar = b.create_block();
            let packed = b.create_block();
            let check_flush = b.create_block();
            let flush = b.create_block();
            let advance = b.create_block();
            let exit = b.create_block();
            b.append_block_params_for_function_params(entry);
            for t in [ptr, ptr, ptr] {
                b.append_block_param(loop_block, t);
                b.append_block_param(advance, t);
                b.append_block_param(check_flush, t);
            }
            b.append_block_param(flush, ptr);
            b.append_block_param(flush, ptr);
            b.append_block_param(flush, ptr);
            let slot = b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (batch * 8) as u32,
                3,
            ));
            b.switch_to_block(entry);
            let params = b.block_params(entry).to_vec();
            let (data, len, map) = (params[0], params[1], params[2]);
            let enough = b
                .ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, len, width as i64);
            let init = b.create_block();
            b.ins().brif(enough, init, &[], exit, &[]);
            b.switch_to_block(init);
            let mut key = b.ins().iconst(ptr, 0);
            for index in 0..width - 1 {
                let byte = b.ins().load(types::I8, MemFlags::new(), data, index as i32);
                let byte = b.ins().uextend(ptr, byte);
                let shifted = b.ins().ishl_imm(key, bits as i64);
                key = b.ins().bor(shifted, byte);
            }
            let index = b.ins().iconst(ptr, (width - 1) as i64);
            let zero = b.ins().iconst(ptr, 0);
            b.ins()
                .jump(loop_block, &[index.into(), key.into(), zero.into()]);
            b.switch_to_block(loop_block);
            let p = b.block_params(loop_block).to_vec();
            let (index, key, used) = (p[0], p[1], p[2]);
            if pack8 && bits == 2 && batch >= 8 && cfg!(target_endian = "little") {
                let end = b.ins().iadd_imm(index, 8);
                let fits_input = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, len);
                let end = b.ins().iadd_imm(used, 8);
                let fits_buffer =
                    b.ins()
                        .icmp_imm(IntCC::UnsignedLessThanOrEqual, end, batch as i64);
                let fits = b.ins().band(fits_input, fits_buffer);
                b.ins().brif(fits, packed, &[], scalar, &[]);
                b.switch_to_block(packed);
                let addr = b.ins().iadd(data, index);
                let word = b.ins().load(ptr, MemFlags::new(), addr, 0);
                let mut word = b.ins().bswap(word);
                for (shift, mask) in [
                    (6, 0x000f000f000f000fu64),
                    (12, 0x000000ff000000ff),
                    (24, 0xffff),
                ] {
                    let shifted = b.ins().ushr_imm(word, shift);
                    word = b.ins().bor(word, shifted);
                    word = b.ins().band_imm(word, mask as i64);
                }
                let buffer = b.ins().stack_addr(ptr, slot, 0);
                let offset = b.ins().ishl_imm(used, 3);
                let addr = b.ins().iadd(buffer, offset);
                let last = if simd {
                    let vector = ptr.by(2).unwrap();
                    let key0 = b.ins().ishl_imm(key, 2);
                    let key1 = b.ins().ishl_imm(key, 4);
                    let old = b.ins().splat(vector, key0);
                    let old = b.ins().insertlane(old, key1, 1);
                    let word1 = b.ins().ishl_imm(word, 2);
                    let words = b.ins().splat(vector, word);
                    let words = b.ins().insertlane(words, word1, 1);
                    let mask = b.ins().iconst(
                        ptr,
                        if width == 32 {
                            -1
                        } else {
                            ((1u64 << (2 * width as u32)) - 1) as i64
                        },
                    );
                    let mask = b.ins().splat(vector, mask);
                    let mut final_pair = old;
                    for pair in 0..4 {
                        let old = b.ins().ishl_imm(old, 4 * pair);
                        let added = b.ins().ushr_imm(words, 14 - 4 * pair);
                        let combined = b.ins().bor(old, added);
                        final_pair = b.ins().band(combined, mask);
                        b.ins()
                            .store(MemFlags::new(), final_pair, addr, 16 * pair as i32);
                    }
                    b.ins().extractlane(final_pair, 1)
                } else {
                    let mut last = key;
                    for j in 0..8 {
                        let old = b.ins().ishl_imm(key, 2 * (j + 1));
                        let added = b.ins().ushr_imm(word, 14 - 2 * j);
                        let combined = b.ins().bor(old, added);
                        last = if width == 32 {
                            combined
                        } else {
                            b.ins()
                                .band_imm(combined, ((1u64 << (2 * width as u32)) - 1) as i64)
                        };
                        b.ins().store(MemFlags::new(), last, addr, 8 * j as i32);
                    }
                    last
                };
                let next = b.ins().iadd_imm(index, 8);
                let used = b.ins().iadd_imm(used, 8);
                b.ins()
                    .jump(check_flush, &[next.into(), last.into(), used.into()]);
                b.switch_to_block(scalar);
            } else {
                b.ins().jump(scalar, &[]);
                b.switch_to_block(packed);
                b.ins()
                    .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
                b.switch_to_block(scalar);
            }
            let addr = b.ins().iadd(data, index);
            let byte = b.ins().load(types::I8, MemFlags::new(), addr, 0);
            let byte = b.ins().uextend(ptr, byte);
            let shifted = b.ins().ishl_imm(key, bits as i64);
            let key = b.ins().bor(shifted, byte);
            let key = if bits as u32 * width as u32 == 64 {
                key
            } else {
                b.ins()
                    .band_imm(key, ((1u64 << (bits as u32 * width as u32)) - 1) as i64)
            };
            let buffer = b.ins().stack_addr(ptr, slot, 0);
            let offset = b.ins().ishl_imm(used, 3);
            let addr = b.ins().iadd(buffer, offset);
            b.ins().store(MemFlags::new(), key, addr, 0);
            let next = b.ins().iadd_imm(index, 1);
            let used = b.ins().iadd_imm(used, 1);
            b.ins()
                .jump(check_flush, &[next.into(), key.into(), used.into()]);
            b.switch_to_block(check_flush);
            let p = b.block_params(check_flush).to_vec();
            let (next, key, used) = (p[0], p[1], p[2]);
            let full = b.ins().icmp_imm(IntCC::Equal, used, batch as i64);
            let end = b.ins().icmp(IntCC::Equal, next, len);
            let should_flush = b.ins().bor(full, end);
            b.ins().brif(
                should_flush,
                flush,
                &[next.into(), key.into(), used.into()],
                advance,
                &[next.into(), key.into(), used.into()],
            );
            b.switch_to_block(flush);
            let p = b.block_params(flush).to_vec();
            let buffer = b.ins().stack_addr(ptr, slot, 0);
            b.ins().call(counter_ref, &[map, buffer, p[2]]);
            let zero = b.ins().iconst(ptr, 0);
            b.ins()
                .jump(advance, &[p[0].into(), p[1].into(), zero.into()]);
            b.switch_to_block(advance);
            let p = b.block_params(advance).to_vec();
            let more = b.ins().icmp(IntCC::UnsignedLessThan, p[0], len);
            b.ins().brif(
                more,
                loop_block,
                &[p[0].into(), p[1].into(), p[2].into()],
                exit,
                &[],
            );
            b.switch_to_block(exit);
            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize();
            module
                .define_function(id, &mut ctx)
                .map_err(|e| e.to_string())?;
            kernels.insert((bits, width), id);
        }
        let mut numeric_kernels = BTreeMap::new();
        for (op, integer) in graphs.iter().flat_map(|(g, _)| {
            g.nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| matches!(n.op, 20..=23))
                .map(|(i, n)| (n.op, g.kinds[g.constants.len() + i] == Kind::Integer))
        }) {
            if numeric_kernels.contains_key(&(op, integer)) {
                continue;
            }
            let mut ctx = module.make_context();
            ctx.func.signature.params = vec![AbiParam::new(ptr); 6];
            if integer {
                ctx.func.signature.returns.push(AbiParam::new(types::I32));
            }
            let id = module
                .declare_function(
                    &format!("mech_numeric_{op}_{}", if integer { "u64" } else { "f64" }),
                    Linkage::Local,
                    &ctx.func.signature,
                )
                .map_err(|e| e.to_string())?;
            let mut fbctx = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let entry = b.create_block();
            let body = b.create_block();
            let exit = b.create_block();
            let error = if integer {
                Some(b.create_block())
            } else {
                None
            };
            if let Some(error) = error {
                b.append_block_param(error, types::I32);
            }
            b.append_block_params_for_function_params(entry);
            b.append_block_param(body, ptr);
            b.switch_to_block(entry);
            let p = b.block_params(entry).to_vec();
            let zero = b.ins().iconst(ptr, 0);
            let more = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, p[5], 0);
            b.ins().brif(more, body, &[zero.into()], exit, &[]);
            b.switch_to_block(body);
            let i = b.block_params(body)[0];
            let scalar1 = b.ins().icmp_imm(IntCC::Equal, p[1], 1);
            let scalar2 = b.ins().icmp_imm(IntCC::Equal, p[3], 1);
            let i1 = b.ins().select(scalar1, zero, i);
            let i2 = b.ins().select(scalar2, zero, i);
            let off1 = b.ins().ishl_imm(i1, 3);
            let off2 = b.ins().ishl_imm(i2, 3);
            let addr1 = b.ins().iadd(p[0], off1);
            let addr2 = b.ins().iadd(p[2], off2);
            let ty = if integer { types::I64 } else { types::F64 };
            let left = b.ins().load(ty, MemFlags::new(), addr1, 0);
            let right = b.ins().load(ty, MemFlags::new(), addr2, 0);
            let value = if integer {
                super::numeric::integer_arithmetic(&mut b, op, left, right, error)
            } else {
                match op {
                    20 => b.ins().fmul(left, right),
                    21 => b.ins().fdiv(left, right),
                    22 => b.ins().fadd(left, right),
                    23 => b.ins().fsub(left, right),
                    _ => unreachable!(),
                }
            };
            let off = b.ins().ishl_imm(i, 3);
            let addr = b.ins().iadd(p[4], off);
            b.ins().store(MemFlags::new(), value, addr, 0);
            let next = b.ins().iadd_imm(i, 1);
            let more = b.ins().icmp(IntCC::UnsignedLessThan, next, p[5]);
            b.ins().brif(more, body, &[next.into()], exit, &[]);
            b.switch_to_block(exit);
            if let Some(error) = error {
                let success = b.ins().iconst(types::I32, 0);
                b.ins().return_(&[success]);
                b.switch_to_block(error);
                let code = b.block_params(error)[0];
                b.ins().return_(&[code]);
            } else {
                b.ins().return_(&[]);
            }
            b.seal_all_blocks();
            b.finalize();
            module
                .define_function(id, &mut ctx)
                .map_err(|e| e.to_string())?;
            numeric_kernels.insert((op, integer), id);
        }
        let mut higher_kernels = BTreeMap::new();
        for (gi, (graph, _)) in graphs.iter().enumerate() {
            for (ni, node) in graph
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| matches!(n.op, 35..=39 | 73))
            {
                let callee = if node.op == 39 {
                    Some(entries[children[&(gi, node.args[2])]])
                } else {
                    None
                };
                let id = super::numeric::emit(
                    module,
                    graph,
                    ni,
                    &format!("mech_array_{gi}_{ni}"),
                    array_simd,
                    callee,
                )?;
                higher_kernels.insert((gi, ni), id);
            }
        }
        // Plan general single-consumer reverse/translation fusion before liveness.
        // Stable handle IDs are retained by a no-op placeholder for the removed node.
        let plans: Vec<_> = graphs
            .iter()
            .map(|(g, output)| {
                let mut nodes = g.nodes.clone();
                let mut uses = BTreeMap::<usize, usize>::new();
                for n in &nodes {
                    for &a in &n.args {
                        *uses.entry(a).or_default() += 1;
                    }
                }
                if std::env::var("MECH_BYTE_FUSION").as_deref() != Ok("0") {
                    for i in 0..nodes.len() {
                        if nodes[i].op != 43 {
                            continue;
                        }
                        let source = nodes[i].args[0];
                        if uses.get(&source) != Some(&1) || Some(source) == *output {
                            continue;
                        }
                        if let Some(j) = source.checked_sub(g.constants.len()) {
                            if nodes[j].op == 44 {
                                nodes[i].op = 46;
                                nodes[i].args[0] = nodes[j].args[0];
                                nodes[j].op = 47;
                                nodes[j].args.clear();
                            }
                        }
                    }
                }
                nodes
            })
            .collect();
        let mut byte_kernels = BTreeMap::new();
        for (gi, (graph, _)) in graphs.iter().enumerate() {
            for (ni, node) in plans[gi].iter().enumerate() {
                let name = format!("mech_bytes_{gi}_{ni}");
                let id = if matches!(node.op, 43 | 44 | 46) {
                    Some(super::bytes::emit(
                        module, graph, &node.args, node.op, &name, array_simd,
                    )?)
                } else if node.op == 49 && array_simd {
                    super::bytes::emit_wrap(module, graph, &node.args, &name)?
                } else if node.op == 45 {
                    Some(super::bytes::emit_records(module, &name)?)
                } else if node.op == 42 {
                    Some(entries[children[&(gi, node.args[1])]])
                } else {
                    None
                };
                if let Some(id) = id {
                    byte_kernels.insert((gi, ni), id);
                }
            }
        }
        for (gi, (graph, output)) in graphs.iter().enumerate() {
            let mut ctx = module.make_context();
            ctx.func.signature = entry_sig.clone();
            let entry_id = entries[gi];
            let dispatch_ref = module.declare_func_in_func(dispatch, &mut ctx.func);
            let refs: BTreeMap<_, _> = kernels
                .iter()
                .map(|(shape, id)| (*shape, module.declare_func_in_func(*id, &mut ctx.func)))
                .collect();
            let numeric_refs: BTreeMap<_, _> = numeric_kernels
                .iter()
                .map(|(op, id)| (*op, module.declare_func_in_func(*id, &mut ctx.func)))
                .collect();
            let higher_refs: BTreeMap<_, _> = higher_kernels
                .iter()
                .filter(|((g, _), _)| *g == gi)
                .map(|((_, n), id)| (*n, module.declare_func_in_func(*id, &mut ctx.func)))
                .collect();
            let byte_refs: BTreeMap<_, _> = byte_kernels
                .iter()
                .filter(|((g, _), _)| *g == gi)
                .map(|((_, n), id)| (*n, module.declare_func_in_func(*id, &mut ctx.func)))
                .collect();
            let mut fbctx = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
            let block = b.create_block();
            b.append_block_params_for_function_params(block);
            b.switch_to_block(block);
            let context = b.block_params(block)[0];
            let mut last_use = BTreeMap::new();
            let mut uses = BTreeMap::<usize, usize>::new();
            for (index, node) in plans[gi].iter().enumerate() {
                for &arg in &node.args {
                    last_use.insert(arg, index);
                    *uses.entry(arg).or_default() += 1;
                }
            }
            // Fuse a single-consumer record extraction with stdin reading. Keep
            // handle allocation order and drain stdin to preserve IO effect order.
            let mut fused = BTreeMap::new();
            for (index, node) in plans[gi].iter().enumerate() {
                if matches!(node.op, 2 | 40) && uses.get(&node.args[0]) == Some(&1) {
                    if let Some(read_index) = node.args[0].checked_sub(graph.constants.len()) {
                        if graph.nodes.get(read_index).is_some_and(|n| n.op == 1) {
                            fused.insert(read_index, 11);
                            fused.insert(index, if node.op == 40 { 41 } else { 10 });
                        }
                    }
                }
            }
            for (index, node) in plans[gi].iter().enumerate() {
                let mut op = fused.get(&index).copied().unwrap_or(node.op);
                if matches!(op, 42 | 43 | 44 | 46)
                    && last_use.get(&node.args[0]) == Some(&index)
                    && Some(node.args[0]) != *output
                    && std::env::var("MECH_BYTE_REUSE").as_deref() != Ok("0")
                {
                    op |= 1u64 << 32;
                }
                let mut args = vec![context, b.ins().iconst(ptr, op as i64)];
                for i in 0..5 {
                    args.push(
                        b.ins()
                            .iconst(ptr, node.args.get(i).copied().unwrap_or(0) as i64),
                    );
                }
                let kernel = if let Some(shape) = node.kernel {
                    b.ins().func_addr(ptr, refs[&shape])
                } else if let Some(reference) = byte_refs.get(&index) {
                    b.ins().func_addr(ptr, *reference)
                } else if let Some(reference) = higher_refs.get(&index) {
                    b.ins().func_addr(ptr, *reference)
                } else if let Some(reference) = numeric_refs.get(&(
                    node.op,
                    graph.kinds[graph.constants.len() + index] == Kind::Integer,
                )) {
                    b.ins().func_addr(ptr, *reference)
                } else {
                    b.ins().iconst(ptr, 0)
                };
                args.push(kernel);
                b.ins().call(dispatch_ref, &args);
                // Release dead intermediates without changing the stable handle IDs.
                let releases: Vec<_> = node
                    .args
                    .iter()
                    .copied()
                    .filter(|id| last_use.get(id) == Some(&index) && Some(*id) != *output)
                    .collect();
                for id in releases
                    .into_iter()
                    .collect::<std::collections::BTreeSet<_>>()
                {
                    let nine = b.ins().iconst(ptr, 9);
                    let handle = b.ins().iconst(ptr, id as i64);
                    let zero = b.ins().iconst(ptr, 0);
                    b.ins().call(
                        dispatch_ref,
                        &[context, nine, handle, zero, zero, zero, zero, zero],
                    );
                }
            }
            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize();
            module
                .define_function(entry_id, &mut ctx)
                .map_err(|e| e.to_string())?;
        }
        Ok(entries[0])
    }
}

/// JIT entry and all higher-order kernels share the AOT definition pass.
pub struct JitProgram {
    _module: cranelift_jit::JITModule,
    entry: super::runtime::NativeEntry,
    constants: Vec<Constant>,
}
impl JitProgram {
    pub fn compile(program: &Program, batch: usize, array_simd: bool) -> Result<Self, String> {
        let mut builder = cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed"), ("is_pic", "true")],
            default_libcall_names(),
        )
        .map_err(|e| e.to_string())?;
        super::runtime::register_symbols(&mut builder);
        let mut module = cranelift_jit::JITModule::new(builder);
        let function = program.define(&mut module, batch, true, false, array_simd)?;
        module.finalize_definitions().map_err(|e| e.to_string())?;
        // SAFETY: shared definition emits NativeEntry, retained by this module.
        let entry = unsafe {
            std::mem::transmute::<*const u8, super::runtime::NativeEntry>(
                module.get_finalized_function(function),
            )
        };
        Ok(Self {
            _module: module,
            entry,
            constants: program.constants.clone(),
        })
    }
    pub fn run(&self, context: &mut super::Context) -> Result<(), String> {
        if !context.matches_constants(&self.constants) {
            return Err("JIT context constant table mismatch".into());
        }
        // SAFETY: the checked constant table binds this context to the code.
        unsafe { context.run(self.entry) }
    }
}
