//! Applicability only for a completed generic String/U64 owner consumed by one
//! already-proved native UTF8 comprehension. No storage or runtime authority.
use super::utf8_reads;
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::{
    NativeControlLoop as Loop, NativeIterationAction as Action, NativeIterationValueScope as Scope,
    NativeMapOwnerWords, NativeMatchAction as Scalar, NativeMatchValue as Value,
    NativeProgramArtifactPlan as Plan, NativeProgramFacts as Facts, NativeProgramReadRole as Read,
    NativeScalarKind as Kind, NativeScalarOp as Op,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Region {
    pub owner: usize,
    pub seed: Value,
    pub result: Value,
    pub exit: u32,
    pub lookup_collection: u32,
    pub lookup_header: u32,
    pub lookup_read: u32,
    pub lookup_exit: u32,
    pub guard: Option<Guard>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
}
impl Layout {
    pub fn exit(&self, site: usize) -> Option<&Region> {
        self.regions.iter().find(|r| r.exit as usize == site)
    }
    pub fn header(&self, site: usize) -> Option<&Region> {
        self.regions
            .iter()
            .find(|r| r.lookup_header as usize == site)
    }
    pub fn extra_work(&self, site: usize) -> u64 {
        self.exit(site)
            .and_then(|r| r.guard.as_ref())
            .map_or(0, Guard::work)
    }
    pub fn derive(
        plan: &Plan,
        owners: &NativeMapOwnerWords,
        reads: &utf8_reads::Layout,
    ) -> Result<Self, &'static str> {
        if owners.regions().is_empty()
            || reads.regions.is_empty()
            || plan.blocks().len() > 16_384
            || plan.values().len() > 65_536
        {
            return Ok(Self::default());
        }
        plan.validate_current_visits()?;
        let facts = plan.facts();
        let mut result = Self::default();
        for (owner, owned) in owners.regions().iter().enumerate() {
            let tail = owned.tail();
            let value = tail.result();
            let parent = tail.parent();
            let exit = tail.exit();
            // Entry into native execution may precede the producer. A boundary
            // after initialization cannot transport a live private owner.
            if let Some(boundary) = plan.activation_boundary()
                && boundary != tail.initialize()
                && !facts.strictly_dominates(boundary, tail.initialize())?
            {
                continue;
            }
            let ty = &plan.values()[value.get() as usize];
            if !ty.definition().dimension_parameters().is_empty()
                || !matches!(ty.definition().body(),SchemaBody::Map{key,value,..}
                    if key.as_ref()==&SchemaBody::String && value.as_ref()==&SchemaBody::UnsignedInteger(IntegerWidth::W64))
                || plan.value_scopes()[value.get() as usize]
                    != (Scope::Immutable { iteration: parent })
                || owners.exit(exit as usize) != Some(owner)
                || plan.blocks()[exit as usize].iteration() != parent
                || !matches!(plan.blocks()[exit as usize].action(),Action::Scalar(Scalar::Copy{source,output})
                    if *source==tail.returned() && *output==value)
            {
                continue;
            }
            let [definition] = facts.definition_sites(value.get())? else {
                continue;
            };
            if definition.site() != Some(exit) {
                continue;
            }
            // The single action role excludes public/structural/frame escapes,
            // effects, captures by other loops, and repeated native consumers.
            let [Read::Action(read_site)] = facts.read_roles(value)? else {
                continue;
            };
            let mut consumers = reads.regions.iter().filter(|r| {
                r.map_root == value.get() && r.map_ordinal.is_none() && r.read_site == *read_site
            });
            let Some(read) = consumers.next() else {
                continue;
            };
            if consumers.next().is_some() {
                continue;
            }
            let Some(Loop::Collection(generator)) = plan.iterations().get(read.iteration as usize)
            else {
                continue;
            };
            if generator.parent() != parent
                || read.project_site.is_some()
                || !facts.strictly_dominates(exit, generator.initialize())?
                || !facts.strictly_dominates(exit, read.header)?
                || plan.activation_boundary() == Some(exit)
                || plan.activation_boundary() == Some(read.header)
                || plan.activation_boundary() == Some(read.read_site)
            {
                continue;
            }
            let guard = if linear_gap(plan, exit, read)? {
                None
            } else {
                let Some(guard) = guarded_gap(plan, exit, read)? else {
                    continue;
                };
                (!guard.conditions.is_empty()).then_some(guard)
            };
            // utf8_reads already proves the complete body, immutable Map
            // capture, exact selected read and ordinary fallback. Do not turn
            // its virtual consumer into an unrelated per-key read hook.
            result.regions.push(Region {
                owner,
                seed: tail.seed(),
                result: value,
                exit,
                lookup_collection: read.collection,
                lookup_header: read.header,
                lookup_read: read.read_site,
                lookup_exit: read.exit,
                guard,
            });
            if result.regions.len() == 256 {
                break;
            }
        }
        Ok(result)
    }
}

fn linear_gap(plan: &Plan, exit: u32, read: &utf8_reads::Region) -> Result<bool, &'static str> {
    use mech_engine::native_artifact::{
        NativeIterationTerminator as T, NativeMatchTerminator as S,
    };
    let T::Scalar(S::Jump(mut at)) = plan.blocks()[exit as usize].terminator() else {
        return Ok(false);
    };
    for _ in 0..plan.blocks().len() {
        if at == read.header {
            return Ok(true);
        }
        let Some(block) = plan.blocks().get(at as usize) else {
            return Err("dictionary gap outside plan");
        };
        if plan.activation_boundary() == Some(at) {
            return Ok(false);
        }
        match block.action() {
            Action::Scalar(Scalar::None | Scalar::Copy { .. }) => {}
            Action::CollectionBegin { collection } if *collection == read.collection => {}
            Action::Initialize { iteration } if *iteration == read.iteration => {}
            _ => return Ok(false),
        }
        let T::Scalar(S::Jump(next)) = block.terminator() else {
            return Ok(false);
        };
        at = next;
    }
    Ok(false)
}

// A bounded, total native prediction chooses publication representation only.
// The source actions, visits, fees and later branch remain in their original CFG.
const MAX_GUARD_SITES: usize = 64;
const MAX_GUARD_ACTIONS: usize = 16;
const MAX_GUARD_BRANCHES: usize = 4;
const MAX_GUARD_LEAVES: usize = 16;
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum GuardAction {
    Copy {
        source: Value,
        output: Value,
    },
    Primitive {
        op: Op,
        operands: Box<[Value]>,
        output: Value,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum GuardCondition {
    Bool {
        condition: Value,
        yes: bool,
    },
    Equal {
        left: Value,
        right: Value,
        yes: bool,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Guard {
    pub actions: Vec<GuardAction>,
    pub conditions: Vec<GuardCondition>,
    pub leaves: Vec<Value>,
    pub sites: Vec<u32>,
}
impl Guard {
    pub fn work(&self) -> u64 {
        // Explicit native leaf loads and total scalar templates are admitted
        // at the original publication visit before anticipatory evaluation.
        self.actions.len() as u64 * 256
            + self.conditions.len() as u64 * 128
            + self.leaves.len() as u64 * 64
            + 256
    }
}
fn total(op: Op) -> bool {
    matches!(
        op,
        Op::U64Eq
            | Op::U64Ne
            | Op::U64Lt
            | Op::U64Le
            | Op::U64Gt
            | Op::U64Ge
            | Op::BoolAnd
            | Op::BoolOr
            | Op::BoolXor
            | Op::BoolNot
            | Op::BoolEq
            | Op::BoolNe
    )
}
fn guard_scalar(plan: &Plan, value: Value) -> bool {
    matches!(
        plan.values()[value.get() as usize].scalar_kind(),
        Some(Kind::U64 | Kind::Bool)
    )
}
fn reaches(plan: &Plan, start: u32, goal: u32) -> Result<bool, &'static str> {
    let mut seen = vec![false; plan.blocks().len()];
    let mut pending = vec![start];
    while let Some(at) = pending.pop() {
        if at == goal {
            return Ok(true);
        }
        let visited = seen
            .get_mut(at as usize)
            .ok_or("dictionary guard edge outside plan")?;
        if *visited {
            continue;
        }
        *visited = true;
        pending.extend(
            Facts::control_successors(plan.blocks()[at as usize].terminator())
                .into_iter()
                .flatten(),
        );
    }
    Ok(false)
}
fn guard_input(
    plan: &Plan,
    facts: &Facts<'_>,
    value: Value,
    exit: u32,
    parent: Option<u32>,
    produced: &std::collections::BTreeSet<u32>,
    leaves: &mut Vec<Value>,
) -> Result<bool, &'static str> {
    if produced.contains(&value.get()) {
        return Ok(true);
    }
    if !guard_scalar(plan, value) {
        return Ok(false);
    }
    let Scope::Immutable { iteration } = plan.value_scopes()[value.get() as usize] else {
        return Ok(false);
    };
    if !facts.lexical_ancestor(iteration, parent) {
        return Ok(false);
    }
    let [definition] = facts.definition_sites(value.get())? else {
        return Ok(false);
    };
    let available = if let Some(site) = definition.site() {
        facts.strictly_dominates(site, exit)?
    } else {
        plan.inputs().iter().any(|p| p.value() == value)
            || plan.constants().iter().any(|c| c.value() == value)
    };
    if !available {
        return Ok(false);
    }
    if !leaves.contains(&value) {
        leaves.push(value);
    }
    Ok(leaves.len() <= MAX_GUARD_LEAVES)
}
fn guard_output(
    plan: &Plan,
    facts: &Facts<'_>,
    value: Value,
    site: u32,
    parent: Option<u32>,
) -> Result<bool, &'static str> {
    if !guard_scalar(plan, value)
        || plan.value_scopes()[value.get() as usize] != (Scope::Immutable { iteration: parent })
    {
        return Ok(false);
    }
    Ok(
        matches!(facts.definition_sites(value.get())?, [definition] if definition.site()==Some(site)),
    )
}
/// A single consumer corridor, with every bypass choosing ordinary publication
/// at the original exit. A second possible consumer route, backedge, effect,
/// checked arithmetic, owner read or activation boundary declines the proof.
fn guarded_gap(
    plan: &Plan,
    exit: u32,
    read: &utf8_reads::Region,
) -> Result<Option<Guard>, &'static str> {
    use mech_engine::native_artifact::{
        NativeIterationTerminator as T, NativeMatchTerminator as S,
    };
    let facts = plan.facts();
    let parent = plan.blocks()[exit as usize].iteration();
    let T::Scalar(S::Jump(mut at)) = plan.blocks()[exit as usize].terminator() else {
        return Ok(None);
    };
    let mut guard = Guard {
        actions: vec![],
        conditions: vec![],
        leaves: vec![],
        sites: vec![],
    };
    let mut produced = std::collections::BTreeSet::new();
    for _ in 0..MAX_GUARD_SITES {
        if at == read.header {
            return Ok(Some(guard));
        }
        let block = plan
            .blocks()
            .get(at as usize)
            .ok_or("dictionary guard site outside plan")?;
        if guard.sites.contains(&at)
            || block.iteration() != parent
            || plan.activation_boundary() == Some(at)
            || !facts.strictly_dominates(exit, at)?
        {
            return Ok(None);
        }
        guard.sites.push(at);
        match block.action() {
            Action::Scalar(Scalar::None) => {}
            Action::Scalar(Scalar::Copy { source, output }) => {
                if !guard_input(
                    plan,
                    &facts,
                    *source,
                    exit,
                    parent,
                    &produced,
                    &mut guard.leaves,
                )? || !guard_output(plan, &facts, *output, at, parent)?
                    || plan.values()[source.get() as usize].scalar_kind()
                        != plan.values()[output.get() as usize].scalar_kind()
                {
                    return Ok(None);
                }
                guard.actions.push(GuardAction::Copy {
                    source: *source,
                    output: *output,
                });
                produced.insert(output.get());
            }
            Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
            }) if total(*op) => {
                if !guard_output(plan, &facts, *output, at, parent)? {
                    return Ok(None);
                }
                for &value in operands.iter() {
                    if !guard_input(
                        plan,
                        &facts,
                        value,
                        exit,
                        parent,
                        &produced,
                        &mut guard.leaves,
                    )? {
                        return Ok(None);
                    }
                }
                guard.actions.push(GuardAction::Primitive {
                    op: *op,
                    operands: operands.clone(),
                    output: *output,
                });
                produced.insert(output.get());
            }
            Action::CollectionBegin { collection } if *collection == read.collection => {}
            Action::Initialize { iteration } if *iteration == read.iteration => {}
            _ => return Ok(None),
        }
        if guard.actions.len() > MAX_GUARD_ACTIONS {
            return Ok(None);
        }
        at = match block.terminator() {
            T::Scalar(S::Jump(next)) => next,
            T::Scalar(S::Branch { condition, yes, no }) => {
                if plan.values()[condition.get() as usize].scalar_kind() != Some(Kind::Bool)
                    || !guard_input(
                        plan,
                        &facts,
                        condition,
                        exit,
                        parent,
                        &produced,
                        &mut guard.leaves,
                    )?
                {
                    return Ok(None);
                }
                let (take_yes, take_no) = (
                    reaches(plan, yes, read.header)?,
                    reaches(plan, no, read.header)?,
                );
                if take_yes == take_no {
                    return Ok(None);
                }
                guard.conditions.push(GuardCondition::Bool {
                    condition,
                    yes: take_yes,
                });
                if take_yes { yes } else { no }
            }
            T::Scalar(S::LiteralBranch {
                left,
                right,
                yes,
                no,
            }) => {
                if plan.values()[left.get() as usize].scalar_kind()
                    != plan.values()[right.get() as usize].scalar_kind()
                    || !guard_input(
                        plan,
                        &facts,
                        left,
                        exit,
                        parent,
                        &produced,
                        &mut guard.leaves,
                    )?
                    || !guard_input(
                        plan,
                        &facts,
                        right,
                        exit,
                        parent,
                        &produced,
                        &mut guard.leaves,
                    )?
                {
                    return Ok(None);
                }
                let (take_yes, take_no) = (
                    reaches(plan, yes, read.header)?,
                    reaches(plan, no, read.header)?,
                );
                if take_yes == take_no {
                    return Ok(None);
                }
                guard.conditions.push(GuardCondition::Equal {
                    left,
                    right,
                    yes: take_yes,
                });
                if take_yes { yes } else { no }
            }
            _ => return Ok(None),
        };
        if guard.conditions.len() > MAX_GUARD_BRANCHES {
            return Ok(None);
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_artifact::program::prepare::matrix_map_keys::tests::artifact;
    fn source() -> String {
        let declarations = super::super::matrix_map_keys::tests::source(
            "at + 1u64",
            "at",
            "at + 1u64",
            "histogram",
        );
        let declarations = declarations.split("\nbuild(").next().unwrap();
        format!(
            "{declarations}\ntags := tags<[string]>\nhistogram := map/histogram(tags)\nkeys := map/keys(histogram)\nend := sequence/length(keys)\nindex := renumber(keys,0u64,end,histogram)\nids := [index{{tag}} | tag <- tags]\nids\n"
        )
    }
    fn derive(a: &mech_engine::ProgramArtifact) -> Layout {
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            a,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        let (_, owners) = plan.facts().buffer_frame_owner_layouts().unwrap();
        let reads = utf8_reads::Layout::derive(&plan).unwrap();
        Layout::derive(&plan, &owners, &reads).unwrap()
    }
    #[test]
    fn generic_dictionary_proves_direct_completed_owner_into_nested_native_lookup() {
        let a = artifact(&source());
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&a).unwrap(),
        )
        .unwrap();
        let proof = derive(&a);
        assert_eq!(proof.regions.len(), 1, "{proof:?}");
        assert_eq!(proof, derive(&decoded));
        assert!(proof.regions[0].exit < proof.regions[0].lookup_header);
    }
    #[test]
    fn generic_dictionary_rejects_public_and_multiple_native_consumer_escapes() {
        let positive = source();
        assert_eq!(derive(&artifact(&positive)).regions.len(), 1);
        let declarations = positive.strip_suffix("ids\n").unwrap();
        let escaping = format!("{declarations}(ids,index)\n");
        assert!(derive(&artifact(&escaping)).regions.is_empty());
        let twice = format!(
            "{declarations}other := other<[string]>\nsecond := [index{{tag}} | tag <- other]\n(ids,second)\n"
        );
        assert!(derive(&artifact(&twice)).regions.is_empty());
    }
    #[test]
    #[ignore = "requires retained benchmark source explicitly supplied by caller"]
    fn generic_dictionary_actual_source_has_two_generic_nonempty_seed_candidates() {
        let s = std::fs::read_to_string(std::env::var("MECH_MATRIX_KEY_BENCH_SOURCE").unwrap())
            .unwrap();
        let proof = derive(&artifact(&s));
        assert_eq!(proof.regions.len(), 2, "{proof:?}");
        let first = proof.regions.iter().find(|p| p.result.get() == 73).unwrap();
        assert_eq!(
            (
                first.owner,
                first.exit,
                first.lookup_collection,
                first.lookup_header,
                first.lookup_read,
                first.lookup_exit
            ),
            (0, 78, 1, 83, 87, 89)
        );
        println!("generic dictionary applicability: {proof:#?}");
    }
    fn guarded_source(guard: &str, inverted: bool) -> String {
        let lookup = "[index{tag} | tag <- tags]";
        let empty = "[0u64 | tag <- tags]";
        let (yes, no) = if inverted {
            (empty, lookup)
        } else {
            (lookup, empty)
        };
        source().replace(
            "ids := [index{tag} | tag <- tags]\nids\n",
            &format!(
                "allowed := {guard}\nids := (allowed? | true => {yes} | false => {no})\nids\n"
            ),
        )
    }
    #[test]
    fn guarded_dictionary_keeps_only_total_single_consumer_corridors_original_and_decoded() {
        for inverted in [false, true] {
            let source = guarded_source("(end >= 3u64) && (end <= 64u64)", inverted);
            let original = artifact(&source);
            let decoded = mech_engine::decode_program_artifact_bytecode_v1(
                &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
            )
            .unwrap();
            let expected = derive(&original);
            assert_eq!(expected.regions.len(), 1, "{expected:?}");
            assert_eq!(expected, derive(&decoded));
            let region = &expected.regions[0];
            let guard = region.guard.as_ref().expect("actual conditional owner");
            assert!(!guard.actions.is_empty());
            assert!(!guard.conditions.is_empty());
            assert!(guard.work() > 0);
            assert_eq!(expected.extra_work(region.exit as usize), guard.work());
            assert_eq!(expected.extra_work(region.lookup_header as usize), 0);
        }
        for unsupported in [
            "(end + 1u64) > 0u64",
            "(end / divisor<u64>) > 0u64",
            "sequence/length(tags) > 0u64",
            "sequence/length([0u64 | at <- 1u64..=end]) > 0u64",
        ] {
            assert!(
                derive(&artifact(&guarded_source(unsupported, false)))
                    .regions
                    .is_empty(),
                "{unsupported}"
            );
        }
        let oversized = (1..=9)
            .map(|n| format!("(end >= {n}u64)"))
            .collect::<Vec<_>>()
            .join(" && ");
        assert!(
            derive(&artifact(&guarded_source(&oversized, false)))
                .regions
                .is_empty(),
            "bounded native guard action limit"
        );
        assert!(
            derive(&artifact(&source())).regions[0].guard.is_none(),
            "unchanged direct corridor needs no native prediction"
        );
    }
}
