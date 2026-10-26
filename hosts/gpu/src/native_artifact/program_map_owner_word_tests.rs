//! Tests only word transport, not a library call, owner loan or full program.
use super::*;
use crate::native_artifact::{abi, memory::NativeWordStorage};
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
use mech_core::ManagedMemoryBudget;
use mech_engine::native_artifact::{
    NativeMapOwnerWords, NativeScalarAggregateAction as Transport, lower_native_program_artifact,
};

type Entry = unsafe extern "C" fn(*mut u64) -> u64;
struct Compiled {
    entry: Entry,
    _module: cranelift_jit::JITModule,
}
impl Compiled {
    fn new(words: &NativeMapOwnerWords, site: usize) -> Self {
        let builder = cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed")],
            default_libcall_names(),
        )
        .unwrap();
        let mut module = cranelift_jit::JITModule::new(builder);
        let mut context = module.make_context();
        context
            .func
            .signature
            .params
            .push(AbiParam::new(types::I64));
        context
            .func
            .signature
            .returns
            .push(AbiParam::new(types::I64));
        let id = module
            .declare_function(
                "owner_word_transport",
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        let mut fb = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let start = b.create_block();
            b.append_block_params_for_function_params(start);
            let failed = b.create_block();
            b.append_block_param(failed, types::I64);
            b.switch_to_block(start);
            let locals = b.block_params(start)[0];
            assert!(emit_memory(&mut b, locals, words.aggregates(), site, failed).unwrap());
            let success = b.ins().iconst(types::I64, abi::STATUS_OK as i64);
            b.ins().return_(&[success]);
            b.switch_to_block(failed);
            let status = b.block_params(failed)[0];
            b.ins().return_(&[status]);
            b.seal_all_blocks();
            b.finalize();
        }
        let ir = context.func.display().to_string();
        assert!(!ir.contains("call ") && !ir.contains("call_indirect"));
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: the definition has exactly one live word-frame pointer and
        // one status result. This owner retains its code through each call.
        let entry =
            unsafe { std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
        Self {
            entry,
            _module: module,
        }
    }
    fn run(&self, storage: &mut NativeWordStorage) -> u64 {
        // SAFETY: the caller allocated every offset in this exact derived
        // frame. Generated transport is synchronous and keeps no pointers.
        unsafe { (self.entry)(storage.values_mut().as_mut_ptr()) }
    }
}
fn artifacts() -> [mech_engine::ProgramArtifact; 2] {
    let catalog = mech_stdlib::source_catalog();
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(catalog)
        .build_compiler()
        .unwrap();
    let source = include_str!("../../tests/fixtures/native/map-owner-word-state.mec");
    let product = compiler.compile_canonical_source(source).unwrap();
    let original = product.artifact().clone();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    [original, decoded]
}

#[test]
fn owner_word_layout_keeps_canonical_entry_exit_and_map_calls_outside_transport() {
    for artifact in artifacts() {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let layout = plan.map_owner_words().unwrap();
        let ordinary = plan.scalar_aggregates();
        assert!(
            ordinary.value_count() > 0,
            "fixture must preserve an existing scalar component"
        );
        for index in 0..plan.values().len() {
            if let Some(slot) = ordinary.slot_at(index) {
                assert_eq!(layout.aggregates().slot_at(index), Some(slot));
            }
        }
        for site in 0..plan.blocks().len() {
            if ordinary.action(site).is_some() {
                assert_eq!(
                    serde_json::to_vec(&ordinary.action(site)).unwrap(),
                    serde_json::to_vec(&layout.aggregates().action(site)).unwrap()
                );
            }
        }
        assert_eq!(
            serde_json::to_vec(ordinary.constants()).unwrap(),
            serde_json::to_vec(layout.aggregates().constants()).unwrap()
        );
        assert_eq!(
            layout.frame_words(),
            plan.values().len() + plan.iterations().len() + layout.aggregates().words()
        );
        assert_eq!(layout.regions().len(), 1);
        let region = &layout.regions()[0];
        let tail = region.tail();
        assert_eq!(layout.initializer(tail.initialize() as usize), Some(0));
        assert_eq!(layout.exit(tail.exit() as usize), Some(0));
        assert!(
            layout
                .aggregates()
                .action(tail.initialize() as usize)
                .is_none()
        );
        assert!(layout.aggregates().action(tail.exit() as usize).is_none());
        assert!(layout.aggregates().slot(tail.seed()).is_none());
        assert!(layout.aggregates().slot(tail.result()).is_none());
        assert!(matches!(
            layout.aggregates().action(tail.latch() as usize),
            Some(Transport::Copy { .. })
        ));
        for &site in region.writes().iter().chain(region.reads()) {
            assert!(layout.aggregates().action(site as usize).is_none());
        }
        for &value in region.values() {
            if matches!(
                plan.values()[value.get() as usize].definition().body(),
                mech_core::SchemaBody::Map { .. }
            ) {
                assert_eq!(
                    layout.aggregates().slot(value),
                    Some((value.get() as usize, 1))
                );
            }
            assert!(
                plan.scalar_aggregates().slot(value).is_none(),
                "default selection changed"
            );
        }
    }
}

#[test]
fn owner_word_relocation_preserves_occupied_prefix_and_emitted_transport() {
    for artifact in artifacts() {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let original = plan.map_owner_words().unwrap();
        let base = original.owner_base();
        let prefix = base + 37;
        let relocated = original.clone().after_frame_words(prefix).unwrap();
        assert_eq!(relocated.owner_base(), prefix);
        assert_eq!(relocated.frame_words(), original.frame_words() + 37);
        assert_eq!(
            relocated.aggregates().words(),
            original.aggregates().words() + 37
        );
        assert!(original.clone().after_frame_words(base - 1).is_err());
        assert!(original.clone().after_frame_words(usize::MAX).is_err());
        assert_eq!(
            serde_json::to_value(relocated.clone().after_frame_words(prefix).unwrap()).unwrap(),
            serde_json::to_value(&relocated).unwrap(),
            "composition is idempotent"
        );
        for index in 0..plan.values().len() {
            let expected = original
                .aggregates()
                .slot_at(index)
                .map(|(at, count)| (if at >= base { at + 37 } else { at }, count));
            assert_eq!(relocated.aggregates().slot_at(index), expected);
        }
        assert_eq!(
            serde_json::to_value(relocated.aggregates().constants()).unwrap(),
            serde_json::to_value(original.aggregates().constants()).unwrap()
        );
        let mut functions = 0;
        for site in 0..plan.blocks().len() {
            let Some(Transport::Project {
                source,
                offset,
                guards,
                output,
            }) = relocated.aggregates().action(site)
            else {
                continue;
            };
            let (src, _) = relocated.aggregates().slot(*source).unwrap();
            let (dst, count) = relocated.aggregates().slot(*output).unwrap();
            let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
            let mut frame =
                NativeWordStorage::new(relocated.frame_words(), 16 * 1024 * 1024, Some(&memory))
                    .unwrap();
            frame.values_mut().fill(0);
            frame.values_mut()[base..prefix].fill(0xfeed_face_dead_beef);
            for &(at, ordinal) in guards {
                frame.values_mut()[src + at] = u64::from(ordinal);
            }
            for index in 0..count {
                frame.values_mut()[src + offset + index] = u64::MAX - index as u64;
            }
            let compiled = Compiled::new(&relocated, site);
            assert_eq!(compiled.run(&mut frame), abi::STATUS_OK);
            for index in 0..count {
                assert_eq!(frame.values()[dst + index], u64::MAX - index as u64);
            }
            assert!(
                frame.values()[base..prefix]
                    .iter()
                    .all(|word| *word == 0xfeed_face_dead_beef),
                "unrelated metadata changed"
            );
            drop(frame);
            assert_eq!(memory.used_bytes(), 0);
            functions += 1;
        }
        assert!(functions > 0);
    }
}

#[test]
fn owner_word_emission_preserves_tokens_and_checks_variant_before_projection() {
    let mut census = [0usize; 4];
    for artifact in artifacts() {
        let plan =
            lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let layout = plan.map_owner_words().unwrap();
        let mut saw_slice = false;
        let mut saw_guard = false;
        for site in 0..plan.blocks().len() {
            let Some(Transport::Project {
                source,
                offset,
                guards,
                output,
            }) = layout.aggregates().action(site)
            else {
                continue;
            };
            let (src, _) = layout.aggregates().slot(*source).unwrap();
            let (dst, count) = layout.aggregates().slot(*output).unwrap();
            let compiled = Compiled::new(&layout, site);
            census[0] += 1;
            census[3] += usize::from(count > 1);
            let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
            let mut frame =
                NativeWordStorage::new(layout.frame_words(), 16 * 1024 * 1024, Some(&memory))
                    .unwrap();
            for token in [1u64, 0x1234_5678_abcd, u64::MAX] {
                frame.values_mut().fill(0);
                for &(word, ordinal) in guards {
                    frame.values_mut()[src + word] = u64::from(ordinal);
                }
                for index in 0..count {
                    frame.values_mut()[src + offset + index] = token.wrapping_add(index as u64);
                }
                assert_eq!(compiled.run(&mut frame), abi::STATUS_OK);
                census[1] += 1;
                for index in 0..count {
                    assert_eq!(
                        frame.values()[dst + index],
                        token.wrapping_add(index as u64)
                    );
                }
                if let Some(&(word, ordinal)) = guards.first() {
                    frame.values_mut()[src + word] = u64::from(ordinal) + 1;
                    frame.values_mut()[dst..dst + count].fill(0xfeed_face);
                    assert_eq!(compiled.run(&mut frame), abi::STATUS_ABI);
                    census[2] += 1;
                    assert!(
                        frame.values()[dst..dst + count]
                            .iter()
                            .all(|&word| word == 0xfeed_face)
                    );
                    saw_guard = true;
                }
            }
            saw_slice |= count > 1;
            drop(frame);
            assert_eq!(memory.used_bytes(), 0);
        }
        assert!(
            saw_slice && saw_guard,
            "fixture must exercise both guarded and multiword transport"
        );
    }
    println!(
        "OWNER_WORD_NATIVE_CASES functions={} success={} guard_denials={} multiword_functions={}",
        census[0], census[1], census[2], census[3]
    );
}
