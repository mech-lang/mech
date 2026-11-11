//! Reuse the canonical histogram/lookup proof with immutable invocation input
//! borrows. Classification and result expansion execute in generated code.
use super::{
    arrays::Arrays,
    collections::Collections,
    flow::Flow,
    layout::{Kind, Layout, Type},
    map_index, packed,
};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::{PreparedNativeProgram, utf8_reads::Region};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, guard},
};
use cranelift_codegen::ir::{
    Block, Endianness, InstBuilder, MemFlags, StackSlot, StackSlotData, StackSlotKind, Value,
    condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
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
        serde_json::json!({"producer":self.producer,"immutable_input":self.source,"lookups":self.regions.iter().map(|r|serde_json::json!({"header":r.header,"read_site":r.read_site})).collect::<Vec<_>>()})
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
        let scalar = Type::derive(&mech_core::SchemaBody::UnsignedInteger(
            mech_core::IntegerWidth::W64,
        ))?;
        let (index, count) = collections.parallel_cursor(b, region.iteration)?;
        let classes = b.ins().stack_load(types::I64, self.slots, 0);
        let initial = b.ins().icmp_imm(IntCC::Equal, index, 0);
        let valid = b.ins().icmp_imm(IntCC::NotEqual, classes, 0);
        let valid = b.ins().band(initial, valid);
        let fast = b.create_block();
        let ordinary = b.create_block();
        b.ins().brif(valid, fast, &[], ordinary, &[]);
        b.switch_to_block(fast);
        let mut output = arrays.allocate(b, count, &scalar, false, error)?;
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
        let offset = b.ins().imul_imm(representative, 8);
        let pointer = b.ins().iadd(output[0], offset);
        // The native classifier points only to the first equal row at/before
        // this one. A prior first row was fully written in this invocation.
        let value = b.ins().load(types::I64, MemFlags::new(), pointer, 0);
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
        b.ins().jump(store, &[value.into()]);
        b.switch_to_block(store);
        let value = b.block_params(store)[0];
        let offset = b.ins().imul_imm(at, 8);
        let pointer = b.ins().iadd(output[0], offset);
        b.ins().store(MemFlags::new(), value, pointer, 0);
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
}
