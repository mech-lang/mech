//! One scalar/SIMD mapping algebra for ordinary and joined mapping leaves.
//! A small dispatch chooses the prepared representation once per invocation.
use super::{NativeCompileError, backend};
use crate::native_artifact::program_ascii_map::{self as plan, Kind};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};

pub(super) struct ScalarTable {
    data: Value,
    kind: Kind,
    domain: Option<(Value, Value)>,
}
impl ScalarTable {
    pub fn new(b: &mut FunctionBuilder<'_>, table: Value, kind: Kind) -> Self {
        let data = b.ins().iadd_imm(table, plan::HEADER as i64);
        let domain = (kind == Kind::Projected).then(|| {
            let mask = b
                .ins()
                .load(types::I8, MemFlags::new(), table, plan::DOMAIN_MASK as i32);
            let expected =
                b.ins()
                    .load(types::I8, MemFlags::new(), table, plan::DOMAIN_VALUE as i32);
            (
                b.ins().uextend(types::I64, mask),
                b.ins().uextend(types::I64, expected),
            )
        });
        Self { data, kind, domain }
    }
    /// The caller supplies the original ASCII guard. Masking also bounds every
    /// private cell access; a rejected domain returns the ordinary missing byte.
    pub fn lookup(&self, b: &mut FunctionBuilder<'_>, key: Value) -> Value {
        let index = b.ins().band_imm(key, self.kind.cells() as i64 - 1);
        let address = b.ins().iadd(self.data, index);
        let value = b.ins().load(types::I8, MemFlags::new(), address, 0);
        if let Some((mask, expected)) = self.domain {
            let domain = b.ins().band(key, mask);
            let valid = b.ins().icmp(IntCC::Equal, domain, expected);
            let missing = b.ins().iconst(types::I8, i64::from(plan::MISSING));
            b.ins().select(valid, value, missing)
        } else {
            value
        }
    }
}

pub(super) struct Tables {
    bands: Vec<Value>,
    offsets: Vec<Value>,
    high_bit: Value,
    domain: Option<(Value, Value, Value)>,
}
impl Tables {
    pub fn new(b: &mut FunctionBuilder<'_>, table: Value, kind: Kind) -> Self {
        let mut bands = Vec::new();
        let mut offsets = Vec::new();
        for index in 0..kind.cells() / 16 {
            bands.push(b.ins().load(
                types::I8X16,
                MemFlags::new(),
                table,
                (plan::HEADER + index * 16) as i32,
            ));
            let offset = b.ins().iconst(types::I8, (index * 16) as i64);
            offsets.push(b.ins().splat(types::I8X16, offset));
        }
        let high = b.ins().iconst(types::I8, i64::from(plan::MISSING));
        let high_bit = b.ins().splat(types::I8X16, high);
        let domain = (kind == Kind::Projected).then(|| {
            let mask = b
                .ins()
                .load(types::I8, MemFlags::new(), table, plan::DOMAIN_MASK as i32);
            let expected =
                b.ins()
                    .load(types::I8, MemFlags::new(), table, plan::DOMAIN_VALUE as i32);
            let mask = b.ins().splat(types::I8X16, mask);
            let expected = b.ins().splat(types::I8X16, expected);
            let low = b.ins().iconst(types::I8, 31);
            (b.ins().splat(types::I8X16, low), mask, expected)
        });
        Self {
            bands,
            offsets,
            high_bit,
            domain,
        }
    }
    pub fn map(&self, b: &mut FunctionBuilder<'_>, keys: Value) -> (Value, Value) {
        let indices = self
            .domain
            .map_or(keys, |(low, _, _)| b.ins().band(keys, low));
        let mut mapped = b.ins().swizzle(self.bands[0], indices);
        for index in 1..self.bands.len() {
            let indices = b.ins().isub(indices, self.offsets[index]);
            let part = b.ins().swizzle(self.bands[index], indices);
            mapped = b.ins().bor(mapped, part);
        }
        // Raw non-ASCII input remains distinct from a missing ASCII key in the
        // joined caller's scalar continuation. No source read gains new flags.
        let invalid = b.ins().bor(mapped, keys);
        let mut invalid = b.ins().band(invalid, self.high_bit);
        if let Some((_, mask, expected)) = self.domain {
            let domain = b.ins().band(keys, mask);
            let valid = b.ins().icmp(IntCC::Equal, domain, expected);
            let outside = b.ins().bnot(valid);
            invalid = b.ins().bor(invalid, outside);
        }
        (mapped, b.ins().vany_true(invalid))
    }
}

/// Account for all emitted alternatives and the nested wrapper+leaf stack.
/// Only one leaf executes; all code remains admitted, including the fallback.
pub(super) fn dispatch<M: Module>(
    module: &mut M,
    symbol: &str,
    full: (FuncId, u64, u64),
    projected: (FuncId, u64, u64),
) -> Result<(FuncId, u64, u64), NativeCompileError> {
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 5];
    signature.returns.push(AbiParam::new(types::I64));
    let function = module
        .declare_function(symbol, Linkage::Local, &signature)
        .map_err(backend)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let full_ref = module.declare_func_in_func(full.0, &mut context.func);
    let projected_ref = module.declare_func_in_func(projected.0, &mut context.func);
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        let ordinary = b.create_block();
        let compact = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let kind = b
            .ins()
            .load(types::I8, MemFlags::new(), args[2], plan::KIND as i32);
        let projected = b.ins().icmp_imm(IntCC::Equal, kind, 1);
        b.ins().brif(projected, compact, &[], ordinary, &[]);
        for (block, callee) in [(ordinary, full_ref), (compact, projected_ref)] {
            b.switch_to_block(block);
            let call = b.ins().call(callee, &args);
            let result = b.inst_results(call)[0];
            b.ins().return_(&[result]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    module
        .define_function(function, &mut context)
        .map_err(backend)?;
    let compiled = context
        .compiled_code()
        .ok_or_else(|| backend("ASCII map dispatch code missing"))?;
    let stack = super::super::compiler::compiled_stack_bytes(&compiled.buffer)?;
    let stack = stack
        .checked_add(full.1.max(projected.1))
        .ok_or_else(|| backend("ASCII map dispatch stack overflow"))?;
    let code = (compiled.code_info().total_size as u64)
        .checked_add(full.2)
        .and_then(|n| n.checked_add(projected.2))
        .ok_or_else(|| backend("ASCII map dispatch code overflow"))?;
    Ok((function, stack, code))
}
