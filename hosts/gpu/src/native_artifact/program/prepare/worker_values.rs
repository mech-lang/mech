//! Physical canonical-owner prefixes for the SAME checked all-path worker CFG.
//! No slot is remapped; unknown storage/effect categories retain the full arena.
use super::{
    ProgramMetadata,
    composition::frames::Source,
    readonly_numeric::{ArrayStorage, ResultRecipe},
    retirement::RetirementKind,
};
use mech_core::SchemaBody;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeIterationValueScope as Scope, NativeMatchTerminator as Branch,
    NativeProgramArtifactPlan as Plan,
};

struct Inventory<'a> {
    plan: &'a Plan,
    prefix: usize,
    supported: bool,
}
impl Inventory<'_> {
    fn value(&mut self, value: u32) -> Result<(), &'static str> {
        let ty = self
            .plan
            .values()
            .get(value as usize)
            .ok_or("worker arena value outside plan")?;
        self.supported &= numeric(ty.definition().body());
        self.prefix = self.prefix.max(value as usize + 1);
        Ok(())
    }
    fn values(&mut self, values: impl IntoIterator<Item = u32>) -> Result<(), &'static str> {
        for value in values {
            self.value(value)?;
        }
        Ok(())
    }
    fn tail(
        &mut self,
        tail: &mech_engine::native_artifact::NativeTailLoop,
    ) -> Result<(), &'static str> {
        self.values([
            tail.seed().get(),
            tail.carrier().get(),
            tail.next().get(),
            tail.returned().get(),
            tail.result().get(),
        ])
    }
}
fn numeric(body: &SchemaBody) -> bool {
    match body {
        SchemaBody::Bool | SchemaBody::Index => true,
        SchemaBody::UnsignedInteger(width) => matches!(
            width,
            mech_core::IntegerWidth::W8
                | mech_core::IntegerWidth::W16
                | mech_core::IntegerWidth::W32
                | mech_core::IntegerWidth::W64
        ),
        SchemaBody::SignedInteger(width) => matches!(width, mech_core::IntegerWidth::W64),
        SchemaBody::FloatingPoint(width) => matches!(width, mech_core::FloatWidth::W64),
        SchemaBody::Matrix { element, .. } => numeric(element),
        SchemaBody::Tuple(fields) => fields.iter().all(numeric),
        SchemaBody::Record(fields) => fields.iter().all(|field| numeric(&field.schema)),
        _ => false,
    }
}
fn under(plan: &Plan, mut owner: u32, root: u32) -> Result<bool, &'static str> {
    for _ in 0..=plan.iterations().len() {
        if owner == root {
            return Ok(true);
        }
        match plan
            .iterations()
            .get(owner as usize)
            .ok_or("worker arena scope outside plan")?
            .as_ref()
            .parent()
        {
            Some(parent) => owner = parent,
            None => return Ok(false),
        }
    }
    Err("worker arena scope parent cycle")
}
/// Recomputed from validated source and private lowering proofs on EVERY image
/// path, including exact Object-bound restoration. Never serialized authority.
pub(in crate::native_artifact) fn derive(
    metadata: &ProgramMetadata,
) -> Result<Box<[usize]>, &'static str> {
    let plan = &metadata.plan;
    let facts = plan.facts();
    let full = metadata.types.len();
    let mut result = Vec::with_capacity(metadata.parallel.regions.len());
    for region in &metadata.parallel.regions {
        let mut inventory = Inventory {
            plan,
            prefix: 0,
            supported: true,
        };
        inventory.values(region.captures.iter().copied())?;
        inventory.values([
            region.source,
            region.output,
            plan.collections()[region.collection as usize]
                .output()
                .get(),
        ])?;
        for &constant in &region.owned_constants {
            inventory.value(
                plan.constants()
                    .get(constant as usize)
                    .ok_or("worker arena constant outside plan")?
                    .value()
                    .get(),
            )?;
        }
        // The complete lexical subtree includes dead/cold locals too. It is
        // intentionally broader than selected successful native operations.
        for (value, scope) in plan.value_scopes().iter().enumerate() {
            let owner = match *scope {
                Scope::Immutable { iteration } => iteration,
                Scope::Carrier { iteration } => Some(iteration),
            };
            if owner
                .map(|owner| under(plan, owner, region.iteration))
                .transpose()?
                .unwrap_or(false)
                || facts
                    .definition_sites(value as u32)?
                    .iter()
                    .any(|definition| definition.site().is_some_and(|site| region.contains(site)))
            {
                inventory.value(value as u32)?;
            }
        }
        for &site in &region.sites {
            facts.parallel_action_reads(site, &mut |value| inventory.value(value.get()))?;
            let block = &plan.blocks()[site as usize];
            match block.terminator() {
                Term::Scalar(Branch::Branch { condition, .. }) => {
                    inventory.value(condition.get())?
                }
                Term::Scalar(Branch::LiteralBranch { left, right, .. }) => {
                    inventory.values([left.get(), right.get()])?
                }
                Term::Scalar(Branch::Complete) => inventory.supported = false,
                _ => {}
            }
            match block.action() {
                Action::CollectionBegin { collection }
                | Action::CollectionFinish { collection } => {
                    let collection = &plan.collections()[*collection as usize];
                    inventory.value(collection.output().get())?;
                    if !matches!(
                        collection.result(),
                        mech_engine::native_artifact::NativeCollectionResult::Materialized
                    ) || collection.string_span_projection().is_some()
                        || collection.string_element_map_projection().is_some()
                    {
                        inventory.supported = false;
                    }
                }
                Action::PrepareStringSpanSelector { .. }
                | Action::ProjectStringSpan { .. }
                | Action::ProjectStringMapEntry { .. }
                | Action::TextJoinFinish { .. }
                | Action::HistogramQueryStep { .. }
                | Action::HistogramFinish { .. } => inventory.supported = false,
                _ => {}
            }
            if let Some(action) = metadata.buffer_frames.action(site as usize) {
                inventory.values(action.retire().iter().map(|value| value.get()))?;
            }
        }
        // These are exactly the cleanup sets the worker's source transitions
        // can execute. Include omitted prepared effects as well as live ones.
        for iteration in 0..plan.iterations().len() {
            if under(plan, iteration as u32, region.iteration)? {
                for kind in [
                    RetirementKind::Collection,
                    RetirementKind::TailLatch,
                    RetirementKind::TailExit,
                ] {
                    for &value in metadata.retirements.slots(
                        metadata.retirements.words(),
                        iteration as u32,
                        kind,
                    )? {
                        inventory.value(
                            u32::try_from(value)
                                .map_err(|_| "worker retirement value exceeds u32")?,
                        )?;
                    }
                }
            }
        }
        // Native entry/continuation metadata may publish a source alias before
        // its ordinary producer. Explicitly include ALL such cold coordinates.
        for lowered in &metadata.closed_tails.regions {
            if !region.contains(lowered.tail.header()) {
                continue;
            }
            inventory.tail(&lowered.tail)?;
            if lowered.borrowed.is_some() || lowered.map.is_some() || lowered.packed.is_some() {
                inventory.supported = false;
            }
            if let Some(proof) = &lowered.numeric {
                if proof
                    .sites
                    .iter()
                    .chain(&proof.prepaid_sites)
                    .any(|&site| !region.contains(site as u32))
                {
                    inventory.supported = false;
                }
                for array in &proof.arrays {
                    match array.storage {
                        ArrayStorage::Capture { value }
                        | ArrayStorage::FrameCapture { value, .. } => inventory.value(value)?,
                        ArrayStorage::Carrier { .. } => {}
                    }
                }
                inventory.values(proof.frames.iter().map(|frame| frame.value))?;
                inventory.values(
                    proof
                        .scalar_inputs
                        .iter()
                        .chain(&proof.scalar_captures)
                        .chain(&proof.scalar_outputs)
                        .map(|binding| binding.value),
                )?;
                match &proof.result {
                    ResultRecipe::Aggregate { value } | ResultRecipe::Frame { value, .. } => {
                        inventory.value(*value)?
                    }
                    _ => {}
                }
                inventory.values(
                    proof
                        .cold_aliases
                        .iter()
                        .flat_map(|(_, aliases)| aliases.iter().map(|alias| alias.value)),
                )?;
                inventory.values(
                    proof
                        .cold_scalars
                        .iter()
                        .flat_map(|(_, bindings)| bindings.iter().map(|binding| binding.value)),
                )?;
                inventory.values(
                    proof
                        .fresh_values
                        .iter()
                        .chain(&proof.boundary_values.candidates)
                        .chain(&proof.boundary_values.header)
                        .chain(&proof.boundary_values.exit)
                        .copied(),
                )?;
                for exit in &proof.exits {
                    if !region.contains(exit.from as u32) || !region.contains(exit.to as u32) {
                        inventory.supported = false;
                    }
                    inventory.values(exit.frames.iter().chain(&exit.values).copied())?;
                    inventory.values(exit.aliases.iter().map(|&(value, _)| value))?;
                }
            }
            if let Some(tree) = &lowered.numeric_tree {
                for node in &tree.nodes {
                    inventory.tail(&node.tail)?;
                    inventory.values(node.scalar_bindings.iter().map(|binding| binding.value))?;
                }
            }
            if let Some(proof) = &lowered.captures {
                inventory.values(proof.captures.iter().map(|capture| capture.value))?;
            }
        }
        for parent in &metadata.compositions.parents {
            for segment in &parent.segments {
                if !region.contains(segment.entry) {
                    continue;
                }
                if segment.sites.iter().any(|&site| !region.contains(site))
                    || segment
                        .exits
                        .iter()
                        .any(|&(from, to)| !region.contains(from) || !region.contains(to))
                {
                    inventory.supported = false;
                }
                if let Some(frames) = &segment.frame_plan {
                    for source in &frames.sources {
                        inventory.value(match *source {
                            Source::Value(value) | Source::Field { value, .. } => value,
                        })?;
                    }
                    inventory.values(frames.aliases.iter().map(|alias| alias.value.get()))?;
                    inventory.values(frames.frames.iter().map(|frame| frame.value))?;
                    inventory.values(frames.inputs.iter().copied())?;
                    inventory.values(frames.scalar_bindings.iter().map(|binding| binding.value))?;
                    inventory.values(
                        frames
                            .retirements
                            .iter()
                            .flat_map(|(_, values)| values.iter().copied()),
                    )?;
                    for exit in &frames.exits {
                        inventory.values(
                            exit.frames
                                .iter()
                                .chain(&exit.aliases)
                                .map(|value| value.get()),
                        )?;
                    }
                }
            }
        }
        result.push(if inventory.supported {
            inventory.prefix
        } else {
            full
        });
    }
    Ok(result.into_boxed_slice())
}
