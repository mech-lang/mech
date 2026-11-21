use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};

mod native_artifact {
    pub type NativeCompileError = String;
    pub mod compiler { pub fn backend(e: impl std::fmt::Display) -> String { e.to_string() } }
    mod compact {
        mod inlining {
            use cranelift_codegen::ir::{ExternalName, FuncRef, Function};
            use cranelift_module::FuncId;
            pub(super) fn module_callee(caller: &Function, callee: FuncRef) -> Option<FuncId> {
                let ExternalName::User(name) = caller.dfg.ext_funcs[callee].name else { return None; };
                let name = &caller.params.user_named_funcs()[name];
                (name.namespace == 0).then(|| FuncId::from_u32(name.index))
            }
        }
        #[path="__ALLOCATION_SETUP_SOURCE__"]
        mod allocation_stubs;
        pub(crate) fn apply<M: cranelift_module::Module>(m: &mut M, f: &mut cranelift_codegen::ir::Function) -> usize {
            allocation_stubs::specialize(m,f).unwrap()
        }
    }
    pub(crate) use compact::apply;
}

fn main() {
    let mut flags=settings::builder(); flags.set("opt_level","speed").unwrap();
    let isa=cranelift_native::builder().unwrap().finish(settings::Flags::new(flags)).unwrap();
    let mut m=ObjectModule::new(ObjectBuilder::new(isa,"literal_setup_abi",default_libcall_names()).unwrap());
    let word=AbiParam::new(types::I64);
    let mut provider=m.make_signature();provider.params.extend([word;5]);provider.returns.extend([word;2]);
    let mut count=0;
    let mut cases=Vec::new();
    for stride in [1,2,4,8] { for zero in [0,1] { cases.push((format!("oracle_{stride}_{zero}"),stride,stride,zero,1,"mech_compact_allocate_backing",false)); } }
    cases.extend([
        ("oracle_duplicate".into(),4,4,1,1,"mech_compact_allocate_backing",false),
        ("oracle_true_bits".into(),4,4,u64::MAX,1,"mech_compact_allocate_backing",false),
        ("oracle_align".into(),4,8,1,0,"mech_compact_allocate_backing",false),
        ("oracle_stride".into(),3,1,1,0,"mech_compact_allocate_backing",false),
        ("oracle_bad_align".into(),4,3,1,0,"mech_compact_allocate_backing",false),
        ("oracle_other_provider".into(),4,4,1,0,"oracle_alternate_allocate",false),
        ("oracle_dynamic".into(),4,4,1,0,"mech_compact_allocate_backing",true),
    ]);
    for (name,stride,alignment,zero,expected,target,dynamic) in cases {
        let mut sig=m.make_signature();sig.params.extend(vec![word;if dynamic{5}else{2}]);sig.returns.extend([word;2]);
        let wrapper=m.declare_function(&name,Linkage::Export,&sig).unwrap();
        let runtime=m.declare_function(target,Linkage::Import,&provider).unwrap();
        let mut ctx=m.make_context();ctx.func.signature=sig;
        let reference=m.declare_func_in_func(runtime,&mut ctx.func);
        // Match PreparedObject's linked direct-runtime calls. An indirect
        // absolute pointer pool in executable text cannot be rebased by dyld.
        ctx.func.dfg.ext_funcs[reference].colocated=true;
        let mut frontend=FunctionBuilderContext::new();
        let mut f=FunctionBuilder::new(&mut ctx.func,&mut frontend);
        let block=f.create_block();f.append_block_params_for_function_params(block);f.switch_to_block(block);
        let inputs=f.block_params(block).to_vec();
        let args=if dynamic { inputs } else {
            let s=f.ins().iconst(types::I64,stride as i64);let a=f.ins().iconst(types::I64,alignment as i64);let z=f.ins().iconst(types::I64,zero as i64);
            vec![inputs[0],inputs[1],s,a,z]
        };
        let call=f.ins().call(reference,&args);let results=f.inst_results(call).to_vec();f.ins().return_(&results);f.seal_all_blocks();f.finalize();
        assert_eq!(native_artifact::apply(&mut m,&mut ctx.func),expected,"{name}");
        ctx.verify(m.isa()).unwrap();m.define_function(wrapper,&mut ctx).unwrap();count+=1;
    }
    std::fs::write(std::env::args().nth(1).unwrap(),m.finish().emit().unwrap()).unwrap();
    println!("Generated {count} independent native ABI wrappers, including repeated geometry and rejected specializations");
}
