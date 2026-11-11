use super::RecordingModule;
use cranelift_codegen::ir::{self, AbiParam, InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};

fn capture(
    name: &str,
    params: &[ir::Type],
    returns: &[ir::Type],
    body: impl FnOnce(&mut FunctionBuilder<'_>),
) -> String {
    let inner = crate::native_artifact::codegen::backend::object("llvm_ssa_test", &[]).unwrap();
    let mut module = RecordingModule::new(inner);
    let mut signature = module.make_signature();
    signature
        .params
        .extend(params.iter().copied().map(AbiParam::new));
    signature
        .returns
        .extend(returns.iter().copied().map(AbiParam::new));
    let id = module
        .declare_function(name, Linkage::Export, &signature)
        .unwrap();
    let mut ctx = module.make_context();
    ctx.func.signature = signature;
    {
        let mut frontend = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut frontend);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        body(&mut b);
        b.seal_all_blocks();
        b.finalize();
    }
    module.define_function(id, &mut ctx).unwrap();
    // A caller may immediately reuse this Context. Captured SSA must survive.
    module.clear_context(&mut ctx);
    module.llvm_ir().unwrap()
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn llvm_ssa_owned_context_and_parallel_edges() {
    let text = capture(
        "choose",
        &[types::I64, types::I64, types::I64],
        &[types::I64],
        |b| {
            let p = b.block_params(b.current_block().unwrap()).to_vec();
            let next = b.create_block();
            b.append_block_param(next, types::I64);
            b.ins()
                .brif(p[0], next, &[p[1].into()], next, &[p[2].into()]);
            b.switch_to_block(next);
            let v = b.block_params(next)[0];
            b.ins().return_(&[v]);
        },
    );
    assert!(text.contains("icmp ne i64 %v0, 0"));
    assert!(text.contains("[ %v1, %edge0 ], [ %v2, %edge1 ]"));
    assert!(!text.contains("inbounds"));
    assert!(!text.contains(" nsw "));
    assert!(!text.contains(" nuw "));
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn llvm_ssa_refuses_unowned_machine_bytes_and_missing_definition() {
    let inner = crate::native_artifact::codegen::backend::object("llvm_ssa_refusal", &[]).unwrap();
    let mut module = RecordingModule::new(inner);
    let sig = module.make_signature();
    let id = module
        .declare_function("missing", Linkage::Local, &sig)
        .unwrap();
    assert!(
        module
            .define_function_bytes(id, 4, &[0, 0, 0, 0], &[])
            .is_err()
    );
    assert!(
        module
            .llvm_ir()
            .unwrap_err()
            .contains("missing LLVM definition")
    );
}

/// Run explicitly with an installed LLVM clang. These are real native execution
/// checks, separate from ordinary compilation and all performance cohorts.
#[test]
#[ignore = "requires MECH_LLVM_TEST_CLANG and an Apple AArch64 host"]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn llvm_ssa_native_integer_boolean_and_vector_semantics() {
    let choose = capture(
        "choose",
        &[types::I64, types::I64, types::I64],
        &[types::I64],
        |b| {
            let p = b.block_params(b.current_block().unwrap()).to_vec();
            let c = b.ins().ireduce(types::I8, p[0]);
            let next = b.create_block();
            b.append_block_param(next, types::I64);
            b.ins().brif(c, next, &[p[1].into()], next, &[p[2].into()]);
            b.switch_to_block(next);
            let v = b.block_params(next)[0];
            b.ins().return_(&[v]);
        },
    );
    let arithmetic = capture(
        "arithmetic",
        &[types::I64, types::I64],
        &[types::I64],
        |b| {
            let p = b.block_params(b.current_block().unwrap()).to_vec();
            let a = b.ins().ishl(p[0], p[1]);
            let z = b.ins().ushr(p[0], p[1]);
            let sum = b.ins().iadd(a, z);
            let pop = b.ins().popcnt(p[0]);
            let out = b.ins().bxor(sum, pop);
            b.ins().return_(&[out]);
        },
    );
    let remainder = capture("test_srem", &[types::I64, types::I64], &[types::I64], |b| {
        let p = b.block_params(b.current_block().unwrap()).to_vec();
        let v = b.ins().srem(p[0], p[1]);
        b.ins().return_(&[v]);
    });
    let vectors = capture("vectors", &[types::I64, types::I64], &[types::I64], |b| {
        let p = b.block_params(b.current_block().unwrap()).to_vec();
        let a = b.ins().load(types::I8X16, ir::MemFlags::new(), p[0], 0);
        let z = b.ins().load(types::I8X16, ir::MemFlags::new(), p[1], 0);
        let same = b.ins().icmp(ir::condcodes::IntCC::Equal, a, z);
        let any = b.ins().vany_true(same);
        let all = b.ins().vall_true(same);
        let all = b.ins().uextend(types::I64, all);
        let all = b.ins().ishl_imm(all, 1);
        let any = b.ins().uextend(types::I64, any);
        let v = b.ins().bor(all, any);
        b.ins().return_(&[v]);
    });
    execute(
        &[choose, arithmetic, remainder, vectors],
        r#"
extern uint64_t choose(uint64_t,uint64_t,uint64_t), arithmetic(uint64_t,uint64_t), test_srem(uint64_t,uint64_t), vectors(uint64_t,uint64_t);
int main(void) {
  uint64_t state=0x1234567812345678ULL;
  for (unsigned i=0;i<10000;i++) {
    state=state*6364136223846793005ULL+1; uint64_t a=state, n=(i*37ULL);
    unsigned s=n&63; uint64_t want=((a<<s)+(a>>s))^(uint64_t)__builtin_popcountll(a);
    if(arithmetic(a,n)!=want)return 1;
    if(choose(i,17,91)!=((i&255)?17:91))return 2;
  }
  if(test_srem(0x8000000000000000ULL,~0ULL)!=0)return 3;
  if(test_srem((uint64_t)-17,5)!=(uint64_t)-2)return 4;
  unsigned char a[16]={0}, b[16]; memset(b,1,16);
  if(vectors((uintptr_t)a,(uintptr_t)b)!=0)return 5;
  b[15]=0;if(vectors((uintptr_t)a,(uintptr_t)b)!=1)return 6;
  memset(b,0,16);if(vectors((uintptr_t)a,(uintptr_t)b)!=3)return 7;
  return 0;
}
"#,
    );
}

#[test]
#[ignore = "requires MECH_LLVM_TEST_CLANG and an Apple AArch64 host"]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn llvm_ssa_native_floating_semantics() {
    let min = capture("minimum", &[types::F64, types::F64], &[types::F64], |b| {
        let p = b.block_params(b.current_block().unwrap()).to_vec();
        let v = b.ins().fmin(p[0], p[1]);
        b.ins().return_(&[v]);
    });
    let max = capture("maximum", &[types::F64, types::F64], &[types::F64], |b| {
        let p = b.block_params(b.current_block().unwrap()).to_vec();
        let v = b.ins().fmax(p[0], p[1]);
        b.ins().return_(&[v]);
    });
    let arithmetic = capture("floating", &[types::F64, types::F64], &[types::F64], |b| {
        let p = b.block_params(b.current_block().unwrap()).to_vec();
        let a = b.ins().fadd(p[0], p[1]);
        let v = b.ins().fneg(a);
        b.ins().return_(&[v]);
    });
    let cast = capture("saturated", &[types::F64], &[types::I64], |b| {
        let p = b.block_params(b.current_block().unwrap())[0];
        let v = b.ins().fcvt_to_sint_sat(types::I64, p);
        b.ins().return_(&[v]);
    });
    execute(
        &[min, max, arithmetic, cast],
        r#"
extern double minimum(double,double), maximum(double,double), floating(double,double);
extern int64_t saturated(double);
static uint64_t bits(double x){uint64_t r;memcpy(&r,&x,8);return r;}
int main(void){
  double x[]={0.0,-0.0,1.0,-1.0,INFINITY,-INFINITY,NAN};
  for(unsigned i=0;i<7;i++)for(unsigned j=0;j<7;j++){
    double a=x[i],b=x[j],lo=minimum(a,b),hi=maximum(a,b);
    if(isnan(a)||isnan(b)){if(!isnan(lo)||!isnan(hi))return 1;}
    else if(a==0&&b==0){
      if(bits(lo)!=((bits(a)|bits(b))&0x8000000000000000ULL))return 2;
      if(bits(hi)!=((bits(a)&bits(b))&0x8000000000000000ULL))return 3;
    }else if(lo!=(a<b?a:b)||hi!=(a>b?a:b))return 4;
    double expected=-(a+b),got=floating(a,b);if(isnan(expected)){if(!isnan(got))return 5;}else if(bits(expected)!=bits(got))return 6;
  }
  if(saturated(NAN)!=0||saturated(INFINITY)!=INT64_MAX||saturated(-INFINITY)!=INT64_MIN)return 7;
  if(saturated(42.9)!=42||saturated(-42.9)!=-42)return 8;
  return 0;
}
"#,
    );
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn execute(modules: &[String], c: &str) {
    use std::{fs, process::Command};
    let clang = std::env::var("MECH_LLVM_TEST_CLANG").expect("explicit LLVM test compiler");
    let sdk = Command::new("/usr/bin/xcrun")
        .arg("--show-sdk-path")
        .output()
        .unwrap();
    assert!(sdk.status.success());
    let sdk = String::from_utf8(sdk.stdout).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut objects = Vec::new();
    for (i, text) in modules.iter().enumerate() {
        let ll = dir.path().join(format!("{i}.ll"));
        let object = dir.path().join(format!("{i}.o"));
        fs::write(&ll, text).unwrap();
        let output = Command::new(&clang)
            .args(["-O3", "-mcpu=apple-m3", "-c"])
            .args(["-isysroot", sdk.trim()])
            .arg(ll)
            .arg("-o")
            .arg(&object)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        objects.push(object);
    }
    let source = dir.path().join("check.c");
    fs::write(
        &source,
        format!("#include <stdint.h>\n#include <string.h>\n#include <math.h>\n{c}"),
    )
    .unwrap();
    let executable = dir.path().join("check");
    let output = Command::new(&clang)
        .args(["-O2", "-ffp-contract=off"])
        .args(["-isysroot", sdk.trim()])
        .arg(source)
        .args(objects)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result = Command::new(executable).output().unwrap();
    assert!(
        result.status.success(),
        "native LLVM semantic check: {:?}\n{}",
        result.status,
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
#[ignore = "requires MECH_LLVM_TEST_CLANG and an Apple AArch64 host"]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn llvm_ssa_native_pair_returns_indirect_calls_and_preserving_bridge() {
    use cranelift_codegen::isa::CallConv;
    use cranelift_module::FuncId;
    type M = RecordingModule<cranelift_object::ObjectModule>;
    fn define(
        m: &mut M,
        id: FuncId,
        sig: ir::Signature,
        body: impl FnOnce(&mut FunctionBuilder<'_>, &mut M),
    ) {
        let mut ctx = m.make_context();
        ctx.func.signature = sig;
        {
            let mut frontend = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut frontend);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            body(&mut b, m);
            b.seal_all_blocks();
            b.finalize();
        }
        m.define_function(id, &mut ctx).unwrap();
    }
    let mut m = RecordingModule::new(
        crate::native_artifact::codegen::backend::object("llvm_ssa_calls", &[]).unwrap(),
    );
    let mut pair_sig = m.make_signature();
    pair_sig.params = vec![AbiParam::new(types::I64); 2];
    pair_sig.returns = vec![AbiParam::new(types::I64); 2];
    let pair = m
        .declare_function("test_pair", Linkage::Export, &pair_sig)
        .unwrap();
    define(&mut m, pair, pair_sig.clone(), |b, _| {
        let p = b.block_params(b.current_block().unwrap()).to_vec();
        let sum = b.ins().iadd(p[0], p[1]);
        let xor = b.ins().bxor(p[0], p[1]);
        b.ins().return_(&[sum, xor]);
    });
    let mut preserve_sig = m.make_signature();
    preserve_sig.call_conv = CallConv::PreserveAll;
    preserve_sig.params.push(AbiParam::new(types::I64));
    let preserve = m
        .declare_function("test_preserving_bridge", Linkage::Local, &preserve_sig)
        .unwrap();
    define(&mut m, preserve, preserve_sig, |b, _| {
        let p = b.block_params(b.current_block().unwrap())[0];
        let v = b.ins().load(types::I64, ir::MemFlags::new(), p, 0);
        let v = b.ins().iadd_imm(v, 3);
        b.ins().store(ir::MemFlags::new(), v, p, 0);
        b.ins().return_(&[]);
    });
    let mut caller_sig = m.make_signature();
    caller_sig.params = vec![AbiParam::new(types::I64); 2];
    caller_sig.returns.push(AbiParam::new(types::I64));
    let caller = m
        .declare_function("test_calls", Linkage::Export, &caller_sig)
        .unwrap();
    define(&mut m, caller, caller_sig, |b, m| {
        let p = b.block_params(b.current_block().unwrap()).to_vec();
        let direct = m.declare_func_in_func(pair, b.func);
        let call = b.ins().call(direct, &p);
        let values = b.inst_results(call).to_vec();
        let address = b.ins().func_addr(types::I64, direct);
        let indirect = b.import_signature(pair_sig);
        let call = b.ins().call_indirect(indirect, address, &p);
        let other = b.inst_results(call).to_vec();
        let slot = b.create_sized_stack_slot(ir::StackSlotData::new(
            ir::StackSlotKind::ExplicitSlot,
            8,
            3,
        ));
        b.ins().stack_store(values[0], slot, 0);
        let address = b.ins().stack_addr(types::I64, slot, 0);
        let bridge = m.declare_func_in_func(preserve, b.func);
        b.ins().call(bridge, &[address]);
        let value = b.ins().stack_load(types::I64, slot, 0);
        let value = b.ins().iadd(value, values[1]);
        let value = b.ins().iadd(value, other[0]);
        let value = b.ins().iadd(value, other[1]);
        b.ins().return_(&[value]);
    });
    let text = m.llvm_ir().unwrap();
    assert!(text.contains("call preserve_allcc void"));
    execute(
        &[text],
        r#"
struct pair {uint64_t sum,xor;};
extern struct pair test_pair(uint64_t,uint64_t);
extern uint64_t test_calls(uint64_t,uint64_t);
int main(void){
  uint64_t state=1;
  for(unsigned i=0;i<10000;i++){
    state=state*6364136223846793005ULL+1;uint64_t a=state,b=~state+(uint64_t)i;
    struct pair p=test_pair(a,b);if(p.sum!=a+b||p.xor!=(a^b))return 1;
    if(test_calls(a,b)!=2*((a+b)+(a^b))+3)return 2;
  }
  return 0;
}
"#,
    );
}
