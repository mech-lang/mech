use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, Block, Function, InstBuilder, MemFlags, Type, Value, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

pub(super) const FIELDS: usize = 16;

fn from_word(b: &mut FunctionBuilder<'_>, value: Value, ty: Type) -> Value {
    let int = match ty.bits() {
        8 => types::I8,
        16 => types::I16,
        32 => types::I32,
        _ => types::I64,
    };
    let value = if int == types::I64 {
        value
    } else {
        b.ins().ireduce(int, value)
    };
    if ty.is_float() {
        b.ins().bitcast(ty, MemFlags::new(), value)
    } else {
        value
    }
}
fn to_word(b: &mut FunctionBuilder<'_>, value: Value) -> Value {
    let ty = b.func.dfg.value_type(value);
    let value = if ty.is_float() {
        b.ins().bitcast(
            if ty.bits() == 32 {
                types::I32
            } else {
                types::I64
            },
            MemFlags::new(),
            value,
        )
    } else {
        value
    };
    if ty.bits() < 64 {
        b.ins().uextend(types::I64, value)
    } else {
        value
    }
}
fn jump(b: &mut FunctionBuilder<'_>, target: Block, args: &[Value]) {
    b.ins().jump(
        target,
        &args.iter().copied().map(Into::into).collect::<Vec<_>>(),
    );
}

/// Modes: unchanged fields; changing inner fields; two outside seeds;
/// irreducible alternate body entry; observable effects and an early failure.
pub(super) fn build(mode: u8, ty: Type) -> (Function, Block) {
    let mut f = Function::new();
    f.signature.params = vec![AbiParam::new(types::I64); 5];
    f.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    let child;
    {
        let mut b = FunctionBuilder::new(&mut f, &mut fc);
        let entry = b.create_block();
        let outer = b.create_block();
        let start = b.create_block();
        child = b.create_block();
        let body = b.create_block();
        let yes = b.create_block();
        let no = b.create_block();
        let join = b.create_block();
        let done = b.create_block();
        let finish = b.create_block();
        b.append_block_params_for_function_params(entry);
        for block in [outer, child, body] {
            for _ in 0..2 {
                b.append_block_param(block, types::I64);
            }
            for _ in 0..FIELDS {
                b.append_block_param(block, ty);
            }
        }
        for _ in 0..FIELDS {
            b.append_block_param(join, ty);
        }
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let zero = b.ins().iconst(types::I64, 0);
        let mut seed = vec![zero, zero];
        for i in 0..FIELDS {
            let word = b.ins().iadd_imm(args[3], i as i64);
            seed.push(from_word(&mut b, word, ty));
        }
        jump(&mut b, outer, &seed);
        b.switch_to_block(outer);
        let parent = b.block_params(outer).to_vec();
        let more = b.ins().icmp(IntCC::UnsignedLessThan, parent[0], args[1]);
        b.ins().brif(more, start, &[], finish, &[]);
        b.switch_to_block(start);
        let mut initial = parent.clone();
        initial[0] = zero;
        if matches!(mode, 2 | 3) {
            let normal = b.create_block();
            let alternate = b.create_block();
            let odd = b.ins().band_imm(parent[0], 1);
            let nonempty = b.ins().icmp_imm(IntCC::NotEqual, args[2], 0);
            let nonempty = b.ins().uextend(types::I64, nonempty);
            let alternate_visit = b.ins().band(odd, nonempty);
            b.ins().brif(alternate_visit, alternate, &[], normal, &[]);
            b.switch_to_block(normal);
            jump(&mut b, child, &initial);
            b.switch_to_block(alternate);
            if mode == 2 {
                for value in &mut initial[2..] {
                    let word = to_word(&mut b, *value);
                    let word = b.ins().iadd_imm(word, 100);
                    *value = from_word(&mut b, word, ty);
                }
            }
            jump(&mut b, if mode == 3 { body } else { child }, &initial);
        } else {
            jump(&mut b, child, &initial);
        }
        b.switch_to_block(child);
        let local = b.block_params(child).to_vec();
        let more = b.ins().icmp(IntCC::UnsignedLessThan, local[0], args[2]);
        b.ins().brif(
            more,
            body,
            &local.iter().copied().map(Into::into).collect::<Vec<_>>(),
            done,
            &[],
        );
        b.switch_to_block(body);
        let current = b.block_params(body).to_vec();
        if mode == 4 {
            let failed = b.create_block();
            let admitted = b.create_block();
            let fail = b.ins().icmp(IntCC::Equal, current[0], args[4]);
            b.ins().brif(fail, failed, &[], admitted, &[]);
            b.switch_to_block(failed);
            b.ins().store(MemFlags::new(), current[0], args[0], 16);
            let bad = b.ins().iconst(types::I64, 0xbad);
            b.ins().return_(&[bad]);
            b.switch_to_block(admitted);
            let visits = b.ins().load(types::I64, MemFlags::new(), args[0], 8);
            let next = b.ins().iadd_imm(visits, 1);
            b.ins().store(MemFlags::new(), next, args[0], 8);
        }
        let parity = b.ins().band_imm(current[0], 1);
        b.ins().brif(parity, yes, &[], no, &[]);
        for block in [yes, no] {
            b.switch_to_block(block);
            jump(&mut b, join, &current[2..]);
        }
        b.switch_to_block(join);
        let mut fields = b.block_params(join).to_vec();
        if mode == 1 {
            for field in &mut fields {
                let word = to_word(&mut b, *field);
                let next = b.ins().iadd_imm(word, 1);
                *field = from_word(&mut b, next, ty);
            }
        }
        let next = b.ins().iadd_imm(current[0], 1);
        let sum = b.ins().iadd(current[1], current[0]);
        let mut carried = vec![next, sum];
        carried.extend(fields);
        jump(&mut b, child, &carried);
        b.switch_to_block(done);
        let next = b.ins().iadd_imm(parent[0], 1);
        let mut sum = local[1];
        for &value in &local[2..] {
            let word = to_word(&mut b, value);
            sum = b.ins().iadd(sum, word);
        }
        let mut carried = vec![next, sum];
        for &value in &parent[2..] {
            let word = to_word(&mut b, value);
            let word = b.ins().iadd_imm(word, 1);
            carried.push(from_word(&mut b, word, ty));
        }
        jump(&mut b, outer, &carried);
        b.switch_to_block(finish);
        if mode == 4 {
            b.ins().store(MemFlags::new(), parent[1], args[0], 0);
        }
        b.ins().return_(&[parent[1]]);
        b.seal_all_blocks();
        b.finalize();
    }
    (f, child)
}
