//! Lazy indices owned by a fresh, read-only lexical collection invocation.
//!
//! A physical address is never a memo key. The current call's borrowed String
//! address is checked only to prevent delegated kernels from substituting a
//! different object for the resolved logical binding.
use super::{budget, general::*};
use crate::text_index::PreparedGraphemeIndex;
use mech_core::{
    ManagedMemoryBudget, ManagedMemoryReservation, MemoryDomainId, ResidentKernelError,
    ResidentKernelInputs, ResidentValueMut, ResidentValueRef,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Owner {
    domain: MemoryDomainId,
    budget: Option<ManagedMemoryBudget>,
}

#[derive(Default)]
struct Active {
    owner: Option<Owner>,
    backing_owner: Option<Owner>,
    scope: Option<Seed>,
    lookup: bool,
    read: Option<(ResidentReadLocation, *const String)>,
    bytes: u64,
    nodes: u64,
}

thread_local! {
    static ACTIVE: RefCell<Active> = RefCell::new(Active::default());
    #[cfg(test)]
    static BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(super) struct OwnerGuard(Option<Owner>);
impl OwnerGuard {
    pub(super) fn enter(domain: MemoryDomainId, budget: Option<ManagedMemoryBudget>) -> Self {
        Self(ACTIVE.with(|active| active.borrow_mut().owner.replace(Owner { domain, budget })))
    }
}
impl Drop for OwnerGuard {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.borrow_mut().owner = self.0.take());
    }
}

/// These bytes are backed by independent live reservations, never by a
/// candidate output credit. The first slice deliberately declines memo
/// creation across nested, different memory domains.
#[derive(Debug)]
struct Backing {
    bytes: u64,
    nodes: u64,
    _reservation: Option<ManagedMemoryReservation>,
}
impl Backing {
    fn reserve(owner: &Owner, bytes: u64, nodes: u64) -> Result<Self, ResidentKernelError> {
        let reservation = owner
            .budget
            .as_ref()
            .map(|budget| budget.reserve_capacity(bytes))
            .transpose()
            .map_err(|_| ResidentKernelError::InvalidShape)?;
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            if active
                .backing_owner
                .as_ref()
                .is_some_and(|current| current != owner)
            {
                return Err(ResidentKernelError::InvalidInput);
            }
            let next_bytes = active
                .bytes
                .checked_add(bytes)
                .ok_or(ResidentKernelError::InvalidShape)?;
            let next_nodes = active
                .nodes
                .checked_add(nodes)
                .ok_or(ResidentKernelError::InvalidShape)?;
            active.backing_owner = Some(owner.clone());
            active.bytes = next_bytes;
            active.nodes = next_nodes;
            Ok(())
        })?;
        Ok(Self {
            bytes,
            nodes,
            _reservation: reservation,
        })
    }
}
impl Drop for Backing {
    fn drop(&mut self) {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            active.bytes = active
                .bytes
                .checked_sub(self.bytes)
                .expect("index backing bytes");
            active.nodes = active
                .nodes
                .checked_sub(self.nodes)
                .expect("index backing nodes");
            if active.bytes == 0 && active.nodes == 0 {
                active.backing_owner = None;
            }
        });
    }
}

/// A latent eligible seed can grow later in this invocation. Fixed plan
/// reuse must not depend on whether the first selector has run yet.
pub(super) fn has_index_scope() -> bool {
    ACTIVE.with(|active| {
        let active = active.borrow();
        active.bytes != 0
            || active
                .scope
                .as_ref()
                .is_some_and(|seed| !seed.control.invariant_text_sources.is_empty())
    })
}

pub(crate) fn live_demand() -> (u64, u64) {
    ACTIVE.with(|active| {
        let active = active.borrow();
        (active.bytes, active.nodes)
    })
}

/// Only the same actual domain and shared account may exclude these already
/// reserved bytes from another temporary reservation. Output credits cannot
/// pay for index backing, and an unrelated account gets no such credit.
pub(super) fn reserved_bytes_for(budget: &ManagedMemoryBudget) -> u64 {
    ACTIVE.with(|active| {
        let active = active.borrow();
        match (&active.owner, &active.backing_owner) {
            (Some(owner), Some(backing))
                if owner == backing && owner.budget.as_ref() == Some(budget) =>
            {
                active.bytes
            }
            _ => 0,
        }
    })
}

struct CachedIndex {
    index: PreparedGraphemeIndex,
    _backing: Backing,
}
struct Entry {
    index: Option<Rc<CachedIndex>>,
}
struct Scope {
    entries: Vec<Entry>,
    _backing: Backing,
}
// This empty seed allocates nothing. First use owns all memo allocation.
struct Seed {
    owner: Owner,
    control: std::sync::Arc<ActivatedComprehensionNode>,
    memo: Option<Rc<RefCell<Scope>>>,
}

pub(super) struct ScopeGuard {
    replaced: bool,
    previous: Option<Seed>,
    lookup: bool,
}
impl Drop for ScopeGuard {
    fn drop(&mut self) {
        let discarded = ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            active.lookup = self.lookup;
            if self.replaced {
                std::mem::replace(&mut active.scope, self.previous.take())
            } else {
                None
            }
        });
        // Backing drops must not re-enter an outstanding ACTIVE borrow.
        drop(discarded);
    }
}

/// A Match/Recur may install another lexical/capture frame. Do not consult
/// an enclosing memo across that boundary; its backing remains live.
pub(super) struct LookupBarrier(bool);
impl LookupBarrier {
    pub(super) fn enter() -> Self {
        Self(ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            std::mem::replace(&mut active.lookup, false)
        }))
    }
}
impl Drop for LookupBarrier {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.borrow_mut().lookup = self.0);
    }
}

fn overlaps(source: ResidentReadLocation, write: ResidentWriteLocation) -> bool {
    let same_storage = matches!(
        (source, write.storage),
        (
            ResidentReadLocation::Constant(_),
            ResidentStorageClass::Constant
        ) | (
            ResidentReadLocation::Input(_) | ResidentReadLocation::LexicalInput(_),
            ResidentStorageClass::Input
        ) | (
            ResidentReadLocation::Scratch(_),
            ResidentStorageClass::Scratch
        )
    );
    if !same_storage || source.region().kind != write.region.kind {
        return false;
    }
    let source = source.region();
    let Some(source_end) = source.offset.checked_add(source.len) else {
        return true;
    };
    let Some(write_end) = write.region.offset.checked_add(write.region.len) else {
        return true;
    };
    source.offset < write_end && write.region.offset < source_end
}

fn readonly_in(
    steps: &[ActivatedTurnStep],
    control: &ActivatedComprehensionNode,
    source: ResidentReadLocation,
    remaining: &mut usize,
    depth: usize,
) -> bool {
    if depth > 32 {
        return false;
    }
    for region in &control.locals {
        if overlaps(
            source,
            ResidentWriteLocation {
                slot: mech_core::CellSlotId::new(0),
                storage: ResidentStorageClass::Scratch,
                region: *region,
            },
        ) {
            return false;
        }
    }
    for step in &control.steps {
        let ActivatedCollectionStep::Operation { node, .. } = step else {
            continue;
        };
        let Some(next) = remaining.checked_sub(1) else {
            return false;
        };
        *remaining = next;
        match &steps[node.get() as usize] {
            ActivatedTurnStep::Kernel(kernel) => {
                if overlaps(source, kernel.write) {
                    return false;
                }
            }
            ActivatedTurnStep::Comprehension(nested) => {
                if overlaps(source, nested.write)
                    || !readonly_in(steps, nested, source, remaining, depth + 1)
                {
                    return false;
                }
            }
            // Until a corresponding lifetime/write proof exists, any such
            // descendant retains the existing uncached selector behavior.
            _ => return false,
        }
    }
    true
}

pub(super) fn qualify_sources(
    steps: &[ActivatedTurnStep],
    reads: &[ResidentReadLocation],
    control: &ActivatedComprehensionNode,
) -> Box<[ResidentReadLocation]> {
    let mut qualified = Vec::new();
    for source in reads[control.reads.start as usize..control.reads.end as usize]
        .iter()
        .chain(control.schema_reads.iter())
        .copied()
    {
        if matches!(source, ResidentReadLocation::State { .. })
            || source.region().kind != mech_core::ResidentValueKind::String
            || source.region().len != 1
            || qualified.contains(&source)
        {
            continue;
        }
        let mut remaining = steps.len();
        if readonly_in(steps, control, source, &mut remaining, 0) {
            qualified.push(source);
        }
    }
    qualified.into_boxed_slice()
}

pub(super) fn enter_collection_scope(
    control: &std::sync::Arc<ActivatedComprehensionNode>,
) -> ScopeGuard {
    let owner = ACTIVE.with(|active| active.borrow().owner.clone());
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let enabled = owner.as_ref().is_some_and(|owner| {
            active
                .backing_owner
                .as_ref()
                .is_none_or(|backing| backing == owner)
        });
        let lookup = std::mem::replace(&mut active.lookup, enabled);
        let seed = owner.map(|owner| Seed {
            owner,
            control: std::sync::Arc::clone(control),
            memo: None,
        });
        ScopeGuard {
            replaced: true,
            previous: std::mem::replace(&mut active.scope, seed),
            lookup,
        }
    })
}

fn selected_scope(
    source: &String,
) -> Result<(Option<(Rc<RefCell<Scope>>, usize)>, usize), ResidentKernelError> {
    let candidate = ACTIVE.with(|active| {
        let active = active.borrow();
        if !active.lookup {
            return None;
        }
        let (location, actual) = active.read?;
        if !std::ptr::eq(actual, source as *const String) {
            return None;
        }
        let seed = active.scope.as_ref()?;
        if active.owner.as_ref() != Some(&seed.owner) {
            return None;
        }
        Some((
            location,
            seed.owner.clone(),
            std::sync::Arc::clone(&seed.control),
            seed.memo.clone(),
        ))
    });
    let Some((location, owner, control, memo)) = candidate else {
        return Ok((None, 0));
    };
    let count = control.invariant_text_sources.len();
    budget::PreparedKernel::new(
        (),
        budget::resident_cost! { compute_work: count, ..budget::KernelCostEstimate::default() },
    )
    .admit()?
    .into_plan();
    let Some(slot) = control
        .invariant_text_sources
        .iter()
        .position(|source| *source == location)
    else {
        return Ok((None, count));
    };
    if let Some(memo) = memo {
        return Ok((Some((memo, slot)), count));
    }
    let metadata = count
        .checked_mul(core::mem::size_of::<Entry>())
        .and_then(|bytes| {
            bytes.checked_add(
                core::mem::size_of::<RefCell<Scope>>() + 2 * core::mem::size_of::<usize>(),
            )
        })
        .ok_or(ResidentKernelError::InvalidShape)?;
    let backing = Backing::reserve(&owner, budget::checked_u64(metadata)?, 2)?;
    budget::PreparedKernel::new(
        (),
        budget::resident_cost! { compute_work: count, ..budget::KernelCostEstimate::default() },
    )
    .admit()?
    .into_plan();
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(count)
        .map_err(|_| ResidentKernelError::InvalidShape)?;
    entries.extend(
        control
            .invariant_text_sources
            .iter()
            .copied()
            .map(|_| Entry { index: None }),
    );
    let memo = Rc::new(RefCell::new(Scope {
        entries,
        _backing: backing,
    }));
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let seed = active
            .scope
            .as_mut()
            .expect("synchronous lexical selection scope");
        seed.memo = Some(Rc::clone(&memo));
    });
    Ok((Some((memo, slot)), count))
}

/// The source reference is held for the complete synchronous dispatch. The
/// pointer is never dereferenced and never enters a memo key.
pub(super) fn execute<'a, I: ResidentKernelInputs>(
    node: &ActivatedKernelNode,
    inputs: &'a I,
    output: ResidentValueMut<'a>,
    reads: &[ResidentReadLocation],
) -> Result<bool, ResidentKernelError> {
    let first_input = if matches!(
        node.construction,
        mech_core::OutputConstruction::ReadModifyWrite { base_input: 0, .. }
    ) {
        node.rmw_base
    } else {
        reads[node.reads.start as usize..node.reads.end as usize]
            .first()
            .copied()
    };
    struct ReadGuard<'a> {
        _source: Option<&'a String>,
        previous: Option<(ResidentReadLocation, *const String)>,
    }
    impl Drop for ReadGuard<'_> {
        fn drop(&mut self) {
            ACTIVE.with(|active| active.borrow_mut().read = self.previous);
        }
    }
    let source = match inputs.get(0) {
        Some(ResidentValueRef::String([source])) => Some(source),
        _ => None,
    };
    let read = first_input
        .zip(source)
        .map(|(location, source)| (location, source as *const String));
    let _guard = ReadGuard {
        _source: source,
        previous: ACTIVE.with(|active| std::mem::replace(&mut active.borrow_mut().read, read)),
    };
    node.kernel.execute(inputs, output)
}

pub(super) struct IndexUse {
    index: Rc<CachedIndex>,
    pub(super) compute_work: usize,
    pub(super) scratch_bytes: usize,
}
impl std::ops::Deref for IndexUse {
    type Target = PreparedGraphemeIndex;
    fn deref(&self) -> &Self::Target {
        &self.index.index
    }
}

/// A miss creates only byte offsets. It never clones or normalizes the source.
pub(super) fn prepared(source: &String) -> Result<IndexUse, ResidentKernelError> {
    let (selected, lookup_work) = selected_scope(source)?;
    if let Some((scope, slot)) = &selected {
        if let Some(index) = &scope.borrow().entries[*slot].index {
            return Ok(IndexUse {
                index: Rc::clone(index),
                compute_work: lookup_work,
                scratch_bytes: 0,
            });
        }
    }
    let count = source
        .len()
        .checked_add(1)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let bytes = count
        .checked_mul(core::mem::size_of::<usize>())
        .and_then(|bytes| {
            bytes.checked_add(
                core::mem::size_of::<CachedIndex>() + 2 * core::mem::size_of::<usize>(),
            )
        })
        .ok_or(ResidentKernelError::InvalidShape)?;
    let owner = ACTIVE.with(|active| active.borrow().owner.clone());
    let backing = if selected.is_some() {
        Some(Backing::reserve(
            owner.as_ref().ok_or(ResidentKernelError::InvalidInput)?,
            budget::checked_u64(bytes)?,
            2,
        )?)
    } else {
        None
    };
    budget::PreparedKernel::new(
        (),
        budget::resident_cost! {
            compute_work: count.checked_add(lookup_work).ok_or(ResidentKernelError::InvalidShape)?,
            temporary_bytes: if backing.is_none() { bytes } else { 0 },
            retained_nodes: if backing.is_none() { 2 } else { 0 },
            ..budget::KernelCostEstimate::default()
        },
    )
    .admit()?
    .into_plan();
    let index = PreparedGraphemeIndex::build_with(
        source,
        |capacity| {
            let mut offsets = Vec::new();
            offsets
                .try_reserve_exact(capacity)
                .map_err(|_| ResidentKernelError::InvalidShape)?;
            Ok(offsets)
        },
        || ResidentKernelError::InvalidShape,
    )?;
    #[cfg(test)]
    BUILDS.with(|builds| builds.set(builds.get() + 1));
    let scratch_bytes = if backing.is_none() { bytes } else { 0 };
    let index = Rc::new(CachedIndex {
        index,
        _backing: backing.unwrap_or(Backing {
            bytes: 0,
            nodes: 0,
            _reservation: None,
        }),
    });
    if let Some((scope, slot)) = selected {
        scope.borrow_mut().entries[slot].index = Some(Rc::clone(&index));
    }
    Ok(IndexUse {
        index,
        compute_work: count
            .checked_add(lookup_work)
            .ok_or(ResidentKernelError::InvalidShape)?,
        scratch_bytes,
    })
}

#[cfg(all(test, feature = "full_source"))]
mod tests {
    use super::*;
    use mech_core::{
        FunctionCatalogBuilder, ReactiveInstanceId, ResidentKernelWorkAccounting, Value, ValueData,
        ValueDataDraft,
        snapshot::{SnapshotValidationContext, ValueDraft},
    };
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };

    fn artifacts(source: &str) -> [crate::ProgramArtifact; 2] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x54494e44), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(
            parsed.is_strictly_clean(),
            "{source}: {:?}",
            parsed.diagnostics
        );
        let original = crate::CanonicalSourceFrontend
            .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
            .unwrap_or_else(|error| panic!("{source}: {error:?}"))
            .compile_artifact()
            .unwrap();
        let decoded = crate::decode_program_artifact_bytecode_v1(
            &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
        )
        .unwrap();
        [original, decoded]
    }

    fn instance(artifact: &crate::ProgramArtifact) -> ReactiveInstance {
        let mut builder = FunctionCatalogBuilder::new();
        crate::install_intrinsic_resident(&mut builder).unwrap();
        activate(
            ReactiveInstanceId::new(0x54494e44, 0),
            artifact,
            &builder.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap()
    }

    fn text_turn(
        instance: &mut ReactiveInstance,
        text: &str,
    ) -> Result<usize, ResidentExecutionError> {
        BUILDS.with(|builds| builds.set(0));
        let slot = instance.plan.inputs[0].slot;
        let result = instance.turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::String(&[text.to_owned()]),
        }]);
        assert_eq!(live_demand(), (0, 0), "lexical backing leaked after turn");
        result?;
        Ok(BUILDS.with(|builds| builds.get()))
    }

    fn strings(value: &Value) -> Vec<String> {
        fn append(data: &ValueData, result: &mut Vec<String>) {
            match data {
                ValueData::String(text) => result.push(text.to_string()),
                ValueData::Matrix(matrix) => {
                    for value in matrix.elements().to_values().iter() {
                        append(&value, result);
                    }
                }
                value => panic!("unexpected collection item {value:?}"),
            }
        }
        let mut result = Vec::new();
        append(value.data(), &mut result);
        result
    }

    #[test]
    fn lexical_string_indices_build_once_and_refresh_equal_byte_length_inputs() {
        for artifact in artifacts("text := signal<string>\n[text[i] | i <- [1u64 2u64]]\n") {
            let mut instance = instance(&artifact);
            for (text, expected) in [
                ("éA", vec!["é", "A"]),
                ("Aé", vec!["A", "é"]),
                ("XYZ", vec!["X", "Y"]),
            ] {
                assert_eq!(text_turn(&mut instance, text).unwrap(), 1);
                assert_eq!(strings(&instance.copied_output(0).unwrap()), expected);
            }
            let old = instance.copied_output(0).unwrap();
            assert!(text_turn(&mut instance, "e\u{301}").is_err());
            assert_eq!(strings(&instance.copied_output(0).unwrap()), strings(&old));
            assert_eq!(text_turn(&mut instance, "éA").unwrap(), 1);
            assert_eq!(strings(&instance.copied_output(0).unwrap()), ["é", "A"]);
        }
    }

    #[test]
    fn overlapping_string_windows_share_one_raw_grapheme_index() {
        for artifact in
            artifacts("text := signal<string>\n[text[i..=(i + 1u64)] | i <- [1u64 2u64]]\n")
        {
            let mut instance = instance(&artifact);
            assert_eq!(text_turn(&mut instance, "e\u{301}🧑‍🚀\r\n").unwrap(), 1);
            assert_eq!(
                strings(&instance.copied_output(0).unwrap()),
                ["e\u{301}🧑‍🚀", "🧑‍🚀\r\n"]
            );
        }
    }

    #[test]
    fn reused_inner_local_gets_a_fresh_defining_scope_even_at_equal_byte_length() {
        for artifact in artifacts("[[word[i] | i <- [1u64 2u64]] | word <- signal<[string]:1,2>]\n")
        {
            let mut instance = instance(&artifact);
            let input = &artifact.inputs()[0];
            let value = ValueDraft {
                schema: input.schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(
                    ["éA", "Aé"]
                        .map(|text| ValueDataDraft::String(text.to_owned()))
                        .into(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            BUILDS.with(|builds| builds.set(0));
            instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot: instance.plan.inputs[0].slot,
                    value: &value,
                }])
                .unwrap()
                .publish()
                .unwrap();
            assert_eq!(BUILDS.with(|builds| builds.get()), 2);
            assert_eq!(
                strings(&instance.copied_output(0).unwrap()),
                ["é", "A", "A", "é"]
            );
            assert_eq!(live_demand(), (0, 0));
        }
    }

    #[test]
    fn false_filter_creates_neither_memo_nor_boundaries() {
        for artifact in artifacts("text := signal<string>\n[text[99u64] | i <- [1u64], false]\n") {
            let mut instance = instance(&artifact);
            assert_eq!(text_turn(&mut instance, "é").unwrap(), 0);
            assert!(strings(&instance.copied_output(0).unwrap()).is_empty());
        }
    }

    #[test]
    fn an_unselected_match_arm_does_not_prepare_string_boundaries() {
        for artifact in artifacts(
            "text := signal<string>\n[(false ? | true => text[99u64] == \"x\" | * => false) | i <- [1u64]]\n",
        ) {
            let mut instance = instance(&artifact);
            assert_eq!(text_turn(&mut instance, "é").unwrap(), 0);
            let output = instance.copied_output(0).unwrap();
            let ValueData::Matrix(matrix) = output.data() else {
                panic!("matrix result");
            };
            assert!(matches!(
                matrix.elements(),
                mech_core::snapshot::SequenceView::Bool([false])
            ));
        }
    }

    #[test]
    fn suspended_capture_indices_use_saved_text_and_end_with_the_resumed_scope() {
        let source = "#Text(value<string>) => <bool>\n  | :Start\n  | :Later.\n#Text(value) -> :Start\n  :Start ~> :Later\n  :Later => \"é\" ∈ {value[i] | i <- [1u64 2u64]}.\n#Text(signal<string>)\n";
        for artifact in artifacts(source) {
            let mut instance = instance(&artifact);
            assert_eq!(text_turn(&mut instance, "éA").unwrap(), 0);
            assert!(instance.has_ready_continuation());
            assert_eq!(text_turn(&mut instance, "Z").unwrap(), 1);
            assert!(matches!(
                instance.copied_output(0).unwrap().data(),
                ValueData::Bool(true)
            ));
        }
    }

    struct Delegate {
        genuine: mech_core::BoundResidentKernel,
        replacement: String,
    }
    fn substituted(
        kernel: &mech_core::BoundResidentKernel,
        inputs: &dyn ResidentKernelInputs,
        output: ResidentValueMut<'_>,
    ) -> Result<bool, ResidentKernelError> {
        struct Replaced<'a>(ResidentValueRef<'a>, ResidentValueRef<'a>);
        impl ResidentKernelInputs for Replaced<'_> {
            fn len(&self) -> usize {
                2
            }
            fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
                match index {
                    0 => Some(self.0),
                    1 => Some(self.1),
                    _ => None,
                }
            }
        }
        let delegate = kernel.retained_state::<Delegate>().unwrap();
        let inputs = Replaced(
            ResidentValueRef::String(std::slice::from_ref(&delegate.replacement)),
            inputs.get(1).ok_or(ResidentKernelError::InvalidInput)?,
        );
        delegate.genuine.execute(&inputs, output)
    }

    #[test]
    fn delegated_selector_with_another_borrowed_string_cannot_reuse_actual_input_index() {
        for artifact in artifacts("text := signal<string>\n[text[i] | i <- [1u64 2u64]]\n") {
            let mut instance = instance(&artifact);
            for step in instance.plan.steps.iter_mut() {
                let ActivatedTurnStep::Kernel(node) = step else {
                    continue;
                };
                if node.kernel.parameters().first() == Some(&1)
                    && node.kernel.parameters().len() == 3
                {
                    let genuine = node.kernel.clone();
                    node.kernel = mech_core::BoundResidentKernel::new(substituted, Box::new([]))
                        .with_retained_state(std::sync::Arc::new(Delegate {
                            genuine,
                            replacement: "Aé".to_owned(),
                        }))
                        .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered);
                }
            }
            assert_eq!(
                text_turn(&mut instance, "éA").unwrap(),
                2,
                "substituted calls must each use their admitted fallback"
            );
            assert_eq!(strings(&instance.copied_output(0).unwrap()), ["A", "é"]);
        }
    }

    #[test]
    fn finite_peak_and_node_checks_include_independently_reserved_index_backing() {
        for (temporary, nodes) in [(511, 3), (1024, 1)] {
            let account = ManagedMemoryBudget::new(2048);
            let domain = mech_core::MemoryDomain::with_memory_budget(account.clone()).unwrap();
            let _owner = OwnerGuard::enter(domain.id(), domain.memory_budget());
            let mut plan =
                budget::detached_turn_plan(mech_core::ResourceDemand::default()).unwrap();
            plan.budget_limits.max_temporary_bytes = Some(temporary);
            plan.budget_limits.max_retained_nodes = Some(nodes);
            budget::with_resident_turn_plan(plan, || {
                let backing = Backing::reserve(
                    &Owner {
                        domain: domain.id(),
                        budget: Some(account.clone()),
                    },
                    512,
                    2,
                )
                .unwrap();
                assert_eq!(account.used_bytes(), 512);
                assert_eq!(reserved_bytes_for(&account), 512);
                assert!(
                    budget::PreparedKernel::new((), budget::KernelCostEstimate::default())
                        .admit()
                        .is_err()
                );
                drop(backing);
                assert_eq!(account.used_bytes(), 0);
            });
            assert_eq!(live_demand(), (0, 0));
        }
    }

    #[test]
    fn configured_backing_claim_is_exclusive_to_its_actual_domain_and_account() {
        let account = ManagedMemoryBudget::new(2048);
        let domain = mech_core::MemoryDomain::with_memory_budget(account.clone()).unwrap();
        let other = mech_core::MemoryDomain::with_memory_budget(account.clone()).unwrap();
        let _owner = OwnerGuard::enter(domain.id(), domain.memory_budget());
        let backing = Backing::reserve(
            &Owner {
                domain: domain.id(),
                budget: Some(account.clone()),
            },
            512,
            2,
        )
        .unwrap();
        assert_eq!(reserved_bytes_for(&account), 512);
        assert_eq!(reserved_bytes_for(&ManagedMemoryBudget::new(2048)), 0);
        {
            let _other_owner = OwnerGuard::enter(other.id(), other.memory_budget());
            assert_eq!(
                reserved_bytes_for(&account),
                0,
                "a shared account is insufficient without the same active domain"
            );
        }
        assert_eq!(reserved_bytes_for(&account), 512);
        drop(backing);
        assert_eq!(account.used_bytes(), 0);
    }

    #[test]
    fn ordinary_initial_plans_include_indices_without_requiring_a_kernel_permit() {
        let account = ManagedMemoryBudget::new(4096);
        let domain = mech_core::MemoryDomain::with_memory_budget(account.clone()).unwrap();
        let _owner = OwnerGuard::enter(domain.id(), domain.memory_budget());
        let owner = Owner {
            domain: domain.id(),
            budget: Some(account.clone()),
        };
        let backing = Backing::reserve(&owner, 512, 2).unwrap();
        let node = mech_core::NodeId::new(0);
        let program = crate::memory_planner::ProgramMemoryPlan {
            budget_limits: mech_core::MemoryBudgetLimits {
                max_temporary_bytes: Some(512),
                max_retained_nodes: Some(2),
                ..mech_core::MemoryBudgetLimits::default()
            },
            ..crate::memory_planner::ProgramMemoryPlan::default()
        };
        let initial = crate::memory_planner::plan_current_resident_turn(
            &program,
            node,
            &crate::memory_planner::TurnMemoryFacts::default(),
        )
        .unwrap();
        assert!(initial.budget_violations.is_empty());
        assert_eq!(initial.demand.turn_peak_bytes, 512);
        assert_eq!(initial.demand.retained_nodes, 2);
        budget::with_resident_turn_plan(initial, || {
            budget::PreparedKernel::new((), budget::KernelCostEstimate::default())
                .admit()
                .unwrap()
                .into_plan();
            let growth = Backing::reserve(&owner, 1, 1).unwrap();
            assert!(
                budget::PreparedKernel::new((), budget::KernelCostEstimate::default())
                    .admit()
                    .is_err()
            );
            drop(growth);
        });
        drop(backing);
        assert_eq!(account.used_bytes(), 0);
    }

    #[test]
    fn lazy_scope_reservation_denial_and_unwind_release_all_index_storage() {
        struct Inputs([String; 1], [u64; 1]);
        impl ResidentKernelInputs for Inputs {
            fn len(&self) -> usize {
                2
            }
            fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
                match index {
                    0 => Some(ResidentValueRef::String(&self.0)),
                    1 => Some(ResidentValueRef::Index(&self.1)),
                    _ => None,
                }
            }
        }
        let artifact = artifacts("text := signal<string>\n[text[i] | i <- [1u64 2u64]]\n")
            .into_iter()
            .next()
            .unwrap();
        let instance = instance(&artifact);
        let control = instance
            .plan
            .steps
            .iter()
            .find_map(|step| match step {
                ActivatedTurnStep::Comprehension(control) => Some(std::sync::Arc::clone(control)),
                _ => None,
            })
            .unwrap();
        let node = instance
            .plan
            .steps
            .iter()
            .find_map(|step| match step {
                ActivatedTurnStep::Kernel(node)
                    if node.kernel.parameters().len() == 3 && node.kernel.parameters()[0] == 1 =>
                {
                    Some(node)
                }
                _ => None,
            })
            .unwrap();
        for limit in [0, 8192] {
            let account = ManagedMemoryBudget::new(limit);
            let domain = mech_core::MemoryDomain::with_memory_budget(account.clone()).unwrap();
            let plan = budget::detached_turn_plan(mech_core::ResourceDemand::default()).unwrap();
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                budget::with_resident_turn_plan(plan, || {
                    let _owner = OwnerGuard::enter(domain.id(), domain.memory_budget());
                    let _scope = enter_collection_scope(&control);
                    assert!(
                        has_index_scope(),
                        "a latent eligible seed disables fixed plans before its first build"
                    );
                    assert_eq!(live_demand(), (0, 0));
                    assert_eq!(
                        account.used_bytes(),
                        0,
                        "scope entry must remain allocation-free"
                    );
                    let input = Inputs(["éA".to_owned()], [1]);
                    let mut output = ["old".to_owned()];
                    BUILDS.with(|builds| builds.set(0));
                    let result = execute(
                        node,
                        &input,
                        ResidentValueMut::String(&mut output),
                        &instance.plan.reads,
                    );
                    if limit == 0 {
                        assert!(result.is_err());
                        assert_eq!(output, ["old"]);
                        assert_eq!(BUILDS.with(|builds| builds.get()), 0);
                    } else {
                        result.unwrap();
                        assert_eq!(output, ["é"]);
                        assert!(account.used_bytes() > 0);
                        panic!("deliberate lexical unwind");
                    }
                })
            }));
            assert_eq!(outcome.is_err(), limit != 0);
            assert_eq!(account.used_bytes(), 0);
            assert_eq!(live_demand(), (0, 0));
        }
    }
}
