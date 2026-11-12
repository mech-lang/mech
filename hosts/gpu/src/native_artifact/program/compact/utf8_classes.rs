//! Reuse the canonical histogram/lookup proof with immutable invocation input
//! borrows. Classification and result expansion execute in generated code.
use super::{
    arrays::Arrays,
    collections::Collections,
    flow::Flow,
    layout::{Kind, Layout},
    map_index, packed,
};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::{PreparedNativeProgram, utf8_reads::Region};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, guard, primitive},
};
use cranelift_codegen::ir::{
    Block, Endianness, InstBuilder, MemFlags, StackSlot, StackSlotData, StackSlotKind, Value,
    condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeScalarOp;
use std::collections::BTreeMap;

#[derive(Clone)]
pub(super) struct Proof {
    pub producer: usize,
    pub source: u32,
    regions: Vec<Region>,
}
impl Proof {
    pub fn derive(prepared: &PreparedNativeProgram, layout: &Layout) -> Vec<Self> {
        let mut groups = BTreeMap::<usize, Self>::new();
        for region in &prepared.metadata().utf8_reads.regions {
            let Some(producer) = region.classification_site else {
                continue;
            };
            // Native input spans are immutable for the complete invocation.
            // Owned/revisited descriptors need a separate lifetime/generation
            // proof before they may reuse this optimization.
            if region.map_ordinal.is_some()
                || !prepared
                    .plan()
                    .inputs()
                    .iter()
                    .any(|p| p.value().get() == region.source)
                || !matches!(layout.ty(region.map_root).kind, Kind::SortedMap { .. })
            {
                continue;
            }
            let group = groups.entry(producer as usize).or_insert_with(|| Self {
                producer: producer as usize,
                source: region.source,
                regions: vec![],
            });
            if group.source == region.source {
                group.regions.push(region.clone());
            }
        }
        groups.into_values().collect()
    }
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({"producer":self.producer,"immutable_input":self.source,"lookups":self.regions.iter().map(|r|serde_json::json!({"header":r.header,"read_site":r.read_site,"conversion_site":r.conversion_site,"output_width":r.output_width})).collect::<Vec<_>>()})
    }
}
pub(super) struct State {
    pub proof: Proof,
    slots: StackSlot,
}
impl State {
    pub fn reserve(b: &mut FunctionBuilder<'_>, proofs: &[Proof], flow: &Flow) -> Vec<Self> {
        let zero = b.ins().iconst(types::I64, 0);
        proofs
            .iter()
            .filter(|p| {
                flow.reachable[p.producer]
                    && p.regions.iter().any(|r| flow.reachable[r.header as usize])
            })
            .map(|proof| {
                let slots = b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    16,
                    3,
                ));
                b.ins().stack_store(zero, slots, 0);
                b.ins().stack_store(zero, slots, 8);
                Self {
                    proof: proof.clone(),
                    slots,
                }
            })
            .collect()
    }
    pub fn publish(
        &self,
        b: &mut FunctionBuilder<'_>,
        arrays: &Arrays,
        view: &[Value],
        error: Block,
    ) {
        let previous = b.ins().stack_load(types::I64, self.slots, 8);
        b.ins().stack_store(view[0], self.slots, 0);
        b.ins().stack_store(view[5], self.slots, 8);
        arrays.free_private(b, previous, error);
    }
    pub fn finish(&self, b: &mut FunctionBuilder<'_>, arrays: &Arrays, error: Block) {
        let owner = b.ins().stack_load(types::I64, self.slots, 8);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().stack_store(zero, self.slots, 0);
        b.ins().stack_store(zero, self.slots, 8);
        arrays.free_private(b, owner, error);
    }
    pub fn at_header(&self, site: usize) -> Option<&Region> {
        self.proof
            .regions
            .iter()
            .find(|r| r.header as usize == site)
    }
    pub fn emit_lookup(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        layout: &Layout,
        arrays: &Arrays,
        collections: &Collections,
        region: &Region,
        exit: Block,
        error: Block,
        endian: Endianness,
    ) -> Result<(), NativeCompileError> {
        let read = |b: &mut FunctionBuilder<'_>, value: u32| {
            (0..layout.ty(value).words)
                .map(|lane| words.read(b, layout.slot(value) + lane))
                .collect::<Vec<_>>()
        };
        let source = read(b, region.source);
        let map = read(b, region.map_root);
        let entry = layout.ty(region.map_root).map_entry()?;
        let fields = entry.fields()?;
        let value_offset = fields[1]
            .packed_offset
            .ok_or_else(|| backend("classified native lookup has no U64 value offset"))?;
        // The same sealed source proof serves both native contracts. Its
        // checked cast is part of the result, not an optional representation.
        let scalar = layout.ty(region.output).element()?;
        let width = match scalar.kind {
            Kind::Scalar {
                bytes,
                signed: false,
                float: false,
            } if bytes == region.output_width && scalar.words == 1 => bytes,
            _ => {
                return Err(backend(
                    "classified native lookup output is not its unsigned source type",
                ));
            }
        };
        let conversion = match (width, region.conversion_site) {
            (1, Some(site)) => Some((NativeScalarOp::U64ToU8, site)),
            (2, Some(site)) => Some((NativeScalarOp::U64ToU16, site)),
            (4, Some(site)) => Some((NativeScalarOp::U64ToU32, site)),
            (8, None) => None,
            _ => {
                return Err(backend(
                    "classified native lookup lost its checked source conversion",
                ));
            }
        };
        let stride = scalar
            .packed_bytes
            .ok_or_else(|| backend("classified native lookup output has no packed stride"))?;
        let (index, count) = collections.parallel_cursor(b, region.iteration)?;
        let classes = b.ins().stack_load(types::I64, self.slots, 0);
        let initial = b.ins().icmp_imm(IntCC::Equal, index, 0);
        let valid = b.ins().icmp_imm(IntCC::NotEqual, classes, 0);
        let valid = b.ins().band(initial, valid);
        let fast = b.create_block();
        let ordinary = b.create_block();
        b.ins().brif(valid, fast, &[], ordinary, &[]);
        b.switch_to_block(fast);
        let mut output = arrays.allocate(b, count, scalar, false, error)?;
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        let head = b.create_block();
        b.append_block_param(head, types::I64);
        let row = b.create_block();
        let reuse = b.create_block();
        let lookup = b.create_block();
        let store = b.create_block();
        b.append_block_param(store, types::I64);
        let done = b.create_block();
        let read_error = b.create_block();
        b.append_block_param(read_error, types::I64);
        b.set_cold_block(read_error);
        let conversion_error = conversion.map(|(_, site)| {
            let error = b.create_block();
            b.append_block_param(error, types::I64);
            b.set_cold_block(error);
            (error, site)
        });
        b.ins().jump(head, &[zero.into()]);
        b.switch_to_block(head);
        let at = b.block_params(head)[0];
        let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
        b.ins().brif(more, row, &[], done, &[]);
        b.switch_to_block(row);
        let offset = b.ins().imul_imm(at, 8);
        let representative = b.ins().iadd(classes, offset);
        let representative = b.ins().load(types::I64, MemFlags::new(), representative, 0);
        let bounded = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, representative, at);
        guard(b, bounded, read_error, 0x19503);
        let previous = b.ins().icmp(IntCC::UnsignedLessThan, representative, at);
        b.ins().brif(previous, reuse, &[], lookup, &[]);
        b.switch_to_block(reuse);
        let offset = b.ins().imul_imm(representative, stride as i64);
        let pointer = b.ins().iadd(output[0], offset);
        // The native classifier points only to the first equal row at/before
        // this one. A prior first row was fully written in this invocation.
        let value = packed::load(b, pointer, scalar)?[0];
        b.ins().jump(store, &[value.into()]);
        b.switch_to_block(lookup);
        let offset = b.ins().imul_imm(at, 16);
        let pointer = b.ins().iadd(source[0], offset);
        let key = packed::load(b, pointer, layout.ty(region.source).element()?)?;
        let (ordinal, present) = map_index::find(
            b,
            &map,
            &key,
            entry.packed_bytes.unwrap(),
            endian,
            read_error,
        );
        guard(b, present, read_error, 0x19501);
        let offset = b
            .ins()
            .imul_imm(ordinal, entry.packed_bytes.unwrap() as i64);
        let pointer = b.ins().iadd(map[0], offset);
        let value = b
            .ins()
            .load(types::I64, MemFlags::new(), pointer, value_offset as i32);
        // A duplicate reuses an earlier already checked result. First rows
        // perform the original checked conversion before any narrowed write.
        let value = if let Some((op, _)) = conversion {
            primitive(b, op, &[value], conversion_error.unwrap().0)
        } else {
            value
        };
        b.ins().jump(store, &[value.into()]);
        b.switch_to_block(store);
        let value = b.block_params(store)[0];
        let offset = b.ins().imul_imm(at, stride as i64);
        let pointer = b.ins().iadd(output[0], offset);
        packed::store(b, pointer, scalar, &[value])?;
        let at = b.ins().iadd_imm(at, 1);
        b.ins().jump(head, &[at.into()]);
        b.switch_to_block(done);
        output[1] = count;
        output[3] = one;
        output[4] = count;
        collections.parallel_result(
            b,
            arrays,
            region.collection,
            region.iteration,
            &output,
            count,
            error,
        )?;
        if self.proof.regions.len() == 1 {
            self.finish(b, arrays, error);
        }
        b.ins().jump(exit, &[]);
        b.switch_to_block(read_error);
        let status = b.block_params(read_error)[0];
        let origin = b.ins().iconst(types::I64, region.read_site as i64);
        b.ins().return_(&[status, origin]);
        if let Some((error, site)) = conversion_error {
            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            let origin = b.ins().iconst(types::I64, site as i64);
            b.ins().return_(&[status, origin]);
        }
        b.switch_to_block(ordinary);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    fn artifacts(source: &str) -> [ProgramArtifact; 2] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x434c415353), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let original = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                mech_stdlib::source_catalog(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        [original, decoded]
    }
    fn proofs(artifact: &ProgramArtifact) -> Vec<Proof> {
        let catalog = mech_stdlib::source_catalog();
        let plan = mech_engine::native_artifact::lower_native_program_artifact(artifact, &catalog)
            .unwrap();
        let prepared = PreparedNativeProgram::prepare(artifact, &catalog, plan).unwrap();
        let layout = Layout::derive(prepared.plan()).unwrap();
        Proof::derive(&prepared, &layout)
    }
    #[test]
    fn input_classes_keep_distinct_immutable_map_reads_in_both_artifact_forms() {
        let source = "+> map\nh := map/histogram(tags<[string]>)\nother := map/insert(h, key<string>, 17u64)\nleft := [h{tag} | tag <- tags]\nright := [other{tag} | tag <- tags]\nanswer := (left,right)\nanswer\n";
        let mut prior = None;
        for artifact in artifacts(source) {
            let proof = proofs(&artifact);
            assert_eq!(proof.len(), 1);
            assert_eq!(proof[0].regions.len(), 2);
            assert_ne!(proof[0].regions[0].map_root, proof[0].regions[1].map_root);
            let summary = proof[0].summary();
            if let Some(prior) = &prior {
                assert_eq!(prior, &summary);
            }
            prior = Some(summary);
        }
    }
    #[test]
    fn absent_unrelated_and_owned_sources_do_not_borrow_input_classification() {
        for source in [
            "+> map\ncounts := index<{string:u64}>\ntags := seed<[string]>\nanswer := [counts{tag} | tag <- tags]\nanswer\n",
            "+> map\ntags := seed<[string]>\nh := map/histogram(other<[string]>)\nanswer := [h{tag} | tag <- tags]\nanswer\n",
            "+> map\ntags := seed<[string]>\nowned := [tags, [\"suffix\"]]\nh := map/histogram(owned)\nanswer := [h{tag} | tag <- owned]\nanswer\n",
        ] {
            for artifact in artifacts(source) {
                assert!(proofs(&artifact).is_empty());
            }
        }
    }
    struct CompactKernel {
        entry: unsafe extern "C" fn(
            *mut crate::native_artifact::compact_runtime::Heap,
            *const u64,
            *mut u64,
        )
            -> crate::native_artifact::compact_runtime::NativeStatus,
        metadata: serde_json::Value,
        conversion_site: Option<u32>,
        _module: cranelift_jit::JITModule,
    }
    impl CompactKernel {
        fn new(artifact: &ProgramArtifact, classified: bool, width: u8) -> Self {
            use crate::native_artifact::compact_runtime as runtime;
            use cranelift_module::{FuncOrDataId, Module, default_libcall_names};
            let catalog = mech_stdlib::source_catalog();
            let plan =
                mech_engine::native_artifact::lower_native_program_artifact(artifact, &catalog)
                    .unwrap();
            let prepared = PreparedNativeProgram::prepare(artifact, &catalog, plan).unwrap();
            let regions = &prepared.metadata().utf8_reads.regions;
            assert_eq!(regions.len(), 1);
            assert_eq!(regions[0].output_width, width);
            assert_eq!(regions[0].classification_site.is_some(), classified);
            let conversion_site = regions[0].conversion_site;
            let mut builder = cranelift_jit::JITBuilder::with_flags(
                &[("opt_level", "speed")],
                default_libcall_names(),
            )
            .unwrap();
            builder.symbol(
                "mech_compact_allocate",
                runtime::mech_compact_allocate as *const u8,
            );
            builder.symbol(
                "mech_compact_writable",
                runtime::mech_compact_writable as *const u8,
            );
            builder.symbol("mech_compact_free", runtime::mech_compact_free as *const u8);
            let mut module = cranelift_jit::JITModule::new(builder);
            let metadata = super::super::define(&mut module, &prepared, false).unwrap();
            assert_eq!(
                metadata["native_string_classes"].as_array().unwrap().len(),
                usize::from(classified)
            );
            if classified {
                assert_eq!(
                    metadata["native_string_classes"][0]["lookups"][0]["output_width"],
                    width
                );
            }
            let output_types = metadata["output_types"].as_array().unwrap();
            assert_eq!(
                output_types.last().unwrap()["kind"]["array_element"]["kind"]["scalar_bytes"],
                width
            );
            let Some(FuncOrDataId::Func(id)) =
                module.get_name(metadata["symbol"].as_str().unwrap())
            else {
                panic!("native compact entry")
            };
            module.finalize_definitions().unwrap();
            // The generated ABI is Heap,input packet,output packet -> two u64 lanes.
            let entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
            Self {
                entry,
                metadata,
                conversion_site,
                _module: module,
            }
        }
        fn run(
            &self,
            tags: &[&str],
            key: &str,
            replacement: u64,
            width: u8,
        ) -> (u64, u64, Vec<u64>) {
            use crate::native_artifact::compact_runtime::Heap;
            let spans = tags
                .iter()
                .map(|tag| [tag.as_ptr() as u64, tag.len() as u64])
                .collect::<Vec<_>>();
            let count = tags.len() as u64;
            let mut inputs = vec![0u64; self.metadata["input_words"].as_u64().unwrap() as usize];
            for port in self.metadata["inputs"].as_array().unwrap() {
                let at = port["input_offset"].as_u64().unwrap() as usize;
                match mech_engine::decode_source_input_name(port["name"].as_str().unwrap())
                    .unwrap()
                    .as_str()
                {
                    "tags" | "other-tags" => inputs[at..at + 6].copy_from_slice(&[
                        spans.as_ptr() as u64,
                        count,
                        count,
                        1,
                        count,
                        0,
                    ]),
                    "key" => {
                        inputs[at..at + 2].copy_from_slice(&[key.as_ptr() as u64, key.len() as u64])
                    }
                    "replacement" => inputs[at] = replacement,
                    name => panic!("unexpected native input {name}"),
                }
            }
            let mut heap = Heap::new();
            let mut output =
                vec![0xa5a5a5a5a5a5a5a5; self.metadata["output_words"].as_u64().unwrap() as usize];
            let prior = output.clone();
            // All borrowed UTF8 spans and packets outlive this synchronous native call.
            let status = unsafe { (self.entry)(&mut heap, inputs.as_ptr(), output.as_mut_ptr()) };
            let values = if status.status == 0 {
                let view = &output[output.len() - 6..];
                assert_eq!(view[1], count);
                assert_eq!(view[3], 1);
                assert_eq!(view[4], count);
                assert!(view[2] >= count);
                (0..tags.len())
                    .map(|at| {
                        let pointer = (view[0] as *const u8).wrapping_add(at * usize::from(width));
                        // The native output layout and initialized extent were checked above.
                        unsafe {
                            match width {
                                1 => u64::from(pointer.read()),
                                2 => u64::from(pointer.cast::<u16>().read_unaligned()),
                                4 => u64::from(pointer.cast::<u32>().read_unaligned()),
                                8 => pointer.cast::<u64>().read_unaligned(),
                                _ => unreachable!(),
                            }
                        }
                    })
                    .collect()
            } else {
                assert_eq!(
                    output, prior,
                    "a failed native cast must not publish a partial packet"
                );
                Vec::new()
            };
            heap.discard_private();
            assert_eq!(heap.counters.live_payload_bytes, 0);
            (status.status, status.origin, values)
        }
    }
    #[test]
    fn compact_typed_classes_match_ordinary_packed_results_checked_origins_and_retry() {
        for (width, maximum) in [(1, 255u64), (2, 65535), (4, u32::MAX as u64), (8, u64::MAX)] {
            for classified in [false, true] {
                let conversion = if width == 8 {
                    String::new()
                } else {
                    format!(
                        "narrow-tag(value<u64>) => (out<u{}>) :=\n  narrowed<u{}> := value\n  out := narrowed.\n",
                        width * 8,
                        width * 8,
                    )
                };
                let histogram = if classified {
                    "tags"
                } else {
                    "other-tags<[string]>"
                };
                let value = if width == 8 {
                    "changed{tag}"
                } else {
                    "narrow-tag(changed{tag})"
                };
                let source = format!(
                    "+> map\n{conversion}tags := tags<[string]>\nh := map/histogram({histogram})\nchanged := map/insert(h,key<string>,replacement<u64>)\nanswer := [{value} | tag <- tags]\nanswer\n"
                );
                for artifact in artifacts(&source) {
                    let kernel = CompactKernel::new(&artifact, classified, width);
                    let (status, _, values) =
                        kernel.run(&["z", "é", "z", "", "é"], "z", maximum, width);
                    assert_eq!(status, 0);
                    assert_eq!(values, vec![maximum, 2, maximum, 1, 2]);
                    let (status, _, values) = kernel.run(&[], "z", maximum, width);
                    assert_eq!(status, 0);
                    assert!(values.is_empty());
                    if width < 8 {
                        let tags = ["a", "é", "🦀", "", "last"];
                        for key in tags {
                            let (status, origin, _) = kernel.run(&tags, key, maximum + 1, width);
                            assert_eq!(status, crate::native_artifact::abi::STATUS_ARITHMETIC);
                            assert_eq!(origin, u64::from(kernel.conversion_site.unwrap()));
                            let (status, _, values) = kernel.run(&tags, key, maximum, width);
                            assert_eq!(status, 0, "successful retry after each failed row");
                            assert_eq!(
                                values,
                                tags.map(|tag| if tag == key { maximum } else { 1 })
                                    .to_vec()
                            );
                        }
                    }
                }
            }
        }
    }
}
