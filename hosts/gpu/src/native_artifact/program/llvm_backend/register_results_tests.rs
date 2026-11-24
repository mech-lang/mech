use super::{lower, recording::Definition};
use cranelift_codegen::{
    Context,
    ir::{self, AbiParam, InstBuilder, types},
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::{path::Path, process::Command};

const STEP: u64 = 0x1234_5678_9abc_def1;
const HASH: u64 = 0x9e37_79b9_7f4a_7c15;

fn object(name: &str) -> ObjectModule {
    let mut flags = settings::builder();
    flags.set("is_pic", "true").unwrap();
    let isa = cranelift_native::builder()
        .unwrap()
        .finish(settings::Flags::new(flags))
        .unwrap();
    ObjectModule::new(ObjectBuilder::new(isa, name, default_libcall_names()).unwrap())
}

fn signature(module: &ObjectModule, words: usize) -> ir::Signature {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(types::I64));
    signature.returns = vec![AbiParam::new(types::I64); words];
    signature
}

fn definition(
    module: &mut ObjectModule,
    id: FuncId,
    signature: ir::Signature,
    returns: usize,
    callee: Option<FuncId>,
) -> Definition {
    let mut context = Context::new();
    context.func.signature = signature;
    let mut frontend = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let input = b.block_params(entry)[0];
    if let Some(callee) = callee {
        let callee = module.declare_func_in_func(callee, b.func);
        let call = b.ins().call(callee, &[input]);
        let results = b.inst_results(call).to_vec();
        assert_eq!(results.len(), returns);
        let mut hash = b.ins().iconst(types::I64, 0);
        for value in results {
            hash = b.ins().bxor(hash, value);
            hash = b.ins().imul_imm(hash, HASH as i64);
        }
        b.ins().return_(&[hash]);
    } else {
        let values = (1..=returns)
            .map(|lane| {
                b.ins()
                    .iadd_imm(input, STEP.wrapping_mul(lane as u64) as i64)
            })
            .collect::<Vec<_>>();
        b.ins().return_(&values);
    }
    b.seal_all_blocks();
    b.finalize();
    Definition {
        id,
        function: context.func,
    }
}

#[test]
fn llvm_register_results_keep_unsupported_layouts_closed() {
    for returns in [
        vec![types::I64; 9],
        vec![types::I64, types::I8],
        vec![types::F64; 2],
    ] {
        let mut module = object("llvm_register_refusal");
        let mut signature = module.make_signature();
        signature.returns = returns.into_iter().map(AbiParam::new).collect();
        module
            .declare_function("unsupported", Linkage::Import, &signature)
            .unwrap();
        assert!(
            lower::module(module.declarations(), std::iter::empty::<&Definition>())
                .unwrap_err()
                .contains("unsupported LLVM multiple-return ABI")
        );
    }
}

/// The public C harness sees only single-word results. Each tested call crosses
/// between separately compiled Cranelift and LLVM functions, so a coincidentally
/// matching LLVM caller/callee ABI cannot hide a register-order mismatch.
#[test]
#[ignore = "requires MECH_LLVM_TEST_BIN and an Apple AArch64 host"]
fn llvm_register_results_interoperate_with_cranelift_in_both_directions() {
    let bin = std::env::var_os("MECH_LLVM_TEST_BIN").expect("select the LLVM bin directory");
    let mut native = object("native_register_results");
    let mut llvm = object("llvm_register_results");
    let mut recorded = Vec::new();
    let mut declarations = String::new();
    let mut checks = String::new();
    for words in 1..=8 {
        let native_name = format!("native_return_{words}");
        let llvm_name = format!("llvm_return_{words}");
        for (module, own, other, caller, is_llvm) in [
            (
                &mut native,
                &native_name,
                &llvm_name,
                format!("native_call_{words}"),
                false,
            ),
            (
                &mut llvm,
                &llvm_name,
                &native_name,
                format!("llvm_call_{words}"),
                true,
            ),
        ] {
            let returns = signature(module, words);
            let own = module
                .declare_function(own, Linkage::Export, &returns)
                .unwrap();
            let other = module
                .declare_function(other, Linkage::Import, &returns)
                .unwrap();
            let single = signature(module, 1);
            let caller = module
                .declare_function(&caller, Linkage::Export, &single)
                .unwrap();
            for definition in [
                definition(module, own, returns, words, None),
                definition(module, caller, single, words, Some(other)),
            ] {
                if is_llvm {
                    recorded.push(definition);
                } else {
                    let mut context = Context::for_function(definition.function);
                    module.define_function(definition.id, &mut context).unwrap();
                }
            }
        }
        declarations.push_str(&format!(
            "extern uint64_t native_call_{words}(uint64_t), llvm_call_{words}(uint64_t);\n"
        ));
        checks.push_str(&format!("if(native_call_{words}(a)!=expected(a,{words}) || llvm_call_{words}(a)!=expected(a,{words})) return {words};\n"));
    }
    let ir = lower::module(llvm.declarations(), recorded.iter()).unwrap();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let out = std::env::temp_dir().join(format!(
        "mech-llvm-register-results-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir(&out).unwrap();
    std::fs::write(out.join("native.o"), native.finish().emit().unwrap()).unwrap();
    std::fs::write(out.join("llvm.ll"), ir).unwrap();
    let c = format!(
        "#include <stdint.h>\n{declarations}\nstatic uint64_t expected(uint64_t a,unsigned n){{uint64_t h=0; for(unsigned i=1;i<=n;i++)h=(h^(a+UINT64_C({STEP})*i))*UINT64_C({HASH}); return h;}}\nint main(void){{ uint64_t state=0; for(unsigned i=0;i<10000;i++){{state=state*UINT64_C(6364136223846793005)+1; uint64_t a=i<64?UINT64_MAX-i:state; {checks} }} return 0;}}\n"
    );
    std::fs::write(out.join("check.c"), c).unwrap();
    let run = |command: &mut Command| {
        let output = command.current_dir(&out).output().unwrap();
        assert!(
            output.status.success(),
            "{command:?}: {}\n{}\nartifacts: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
            out.display()
        );
    };
    run(Command::new(Path::new(&bin).join("llc")).args([
        "-O=2",
        "-mcpu=apple-m1",
        "-filetype=obj",
        "-relocation-model=pic",
        "llvm.ll",
        "-o",
        "llvm.o",
    ]));
    run(Command::new("cc").args(["-O2", "check.c", "native.o", "llvm.o", "-o", "check"]));
    run(&mut Command::new(out.join("check")));
    std::fs::remove_dir_all(out).unwrap();
}
