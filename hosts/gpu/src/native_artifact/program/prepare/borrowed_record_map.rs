//! A canonical publication may lend one opaque Map field to its next read.
//! The root stays canonical; no nested Value or mutable owner token is forged.
use mech_core::{IntegerWidth, SchemaBody, ValueData};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as P, NativeIterationAction as A, NativeIterationTerminator as T,
    NativeIterationValueScope as Scope, NativeMapOwnerWords, NativeMatchTerminator as S,
    NativeProgramArtifactPlan as Plan, NativeProgramReadRole as Read,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Region {
    pub owner: usize,
    pub exit: usize,
    pub source: u32,
    pub ordinal: usize,
    pub output: u32,
    pub project: usize,
    pub read: usize,
    pub iteration: Option<u32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
    pub projects: Vec<Option<usize>>,
    pub reads: Vec<Option<usize>>,
}

fn string_u64_map(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Map {key,value,..}
        if key.as_ref()==&SchemaBody::String
        && value.as_ref()==&SchemaBody::UnsignedInteger(IntegerWidth::W64))
}

impl Layout {
    pub fn derive(plan: &Plan, owners: &NativeMapOwnerWords) -> Result<Self, &'static str> {
        plan.validate_current_visits()?;
        if plan.blocks().len() > 16_384 || plan.values().len() > 65_536 {
            return Ok(Self::default());
        }
        let mut out = Self {
            projects: vec![None; plan.blocks().len()],
            reads: vec![None; plan.blocks().len()],
            ..Self::default()
        };
        let facts = plan.facts();
        for (project, block) in plan.blocks().iter().enumerate() {
            if out.regions.len() == 256 {
                break;
            }
            let A::LibraryCall {
                template,
                operands,
                output,
            } = block.action()
            else {
                continue;
            };
            if template.primitive() != P::RecordFieldAccess
                || operands.len() != 2
                || !plan.values()[output.get() as usize]
                    .definition()
                    .dimension_parameters()
                    .is_empty()
                || !string_u64_map(template.output().body())
                || template.inputs().len() != 2
                || template.inputs()[1].body() != &SchemaBody::Id
            {
                continue;
            }
            let source = operands[0];
            let Some((owner, owned)) = owners
                .regions()
                .iter()
                .enumerate()
                .find(|(_, r)| r.tail().result() == source)
            else {
                continue;
            };
            let exit = owned.tail().exit() as usize;
            let [source_definition] = facts.definition_sites(source.get())? else {
                continue;
            };
            let [project_definition] = facts.definition_sites(output.get())? else {
                continue;
            };
            if owners.exit(exit) != Some(owner)
                || source_definition.site() != Some(exit as u32)
                || project_definition.site() != Some(project as u32)
                || !facts.lexical_ancestor(owned.tail().parent(), block.iteration())
                || !facts.strictly_dominates(exit as u32, project as u32)?
                || plan.value_scopes()[source.get() as usize]
                    != (Scope::Immutable {
                        iteration: owned.tail().parent(),
                    })
                || !plan.values()[source.get() as usize]
                    .definition()
                    .dimension_parameters()
                    .is_empty()
                || &template.inputs()[0] != plan.values()[source.get() as usize].definition()
            {
                continue;
            }
            let SchemaBody::Record(fields) = template.inputs()[0].body() else {
                continue;
            };
            if fields.is_empty()
                || fields.len() > 32
                || fields.iter().filter(|f| string_u64_map(&f.schema)).count() != 1
                || fields.iter().any(|f| {
                    !string_u64_map(&f.schema)
                        && f.schema != SchemaBody::UnsignedInteger(IntegerWidth::W64)
                })
            {
                continue;
            }
            let Some(ValueData::Id(selector)) = plan
                .constants()
                .iter()
                .find(|c| c.value() == operands[1])
                .map(|c| c.constant().data())
            else {
                continue;
            };
            let mut matching = fields
                .iter()
                .enumerate()
                .filter(|(_, f)| mech_core::hash_str(&f.name) == *selector);
            let Some((ordinal, field)) = matching.next() else {
                continue;
            };
            if matching.next().is_some() || &field.schema != template.output().body() {
                continue;
            }
            // This complete inventory includes hidden collection/control uses,
            // public outputs and effects, not just visible library operands.
            let [Read::Action(read)] = facts.read_roles(*output)? else {
                continue;
            };
            let read = *read as usize;
            if block.terminator() != T::Scalar(S::Jump(read as u32))
                || facts.predecessors(read as u32)? != [project as u32]
                || plan.blocks()[read].iteration() != block.iteration()
                || plan.activation_boundary() == Some(read as u32)
                || plan.activation_boundary() == Some(project as u32)
            {
                continue;
            }
            let A::LibraryCall {
                template: call,
                operands: inputs,
                output: result,
            } = plan.blocks()[read].action()
            else {
                continue;
            };
            if !matches!(call.primitive(), P::MapKeyAccess | P::MapContainsKey)
                || inputs.len() != 2
                || inputs[0] != *output
                || inputs[1] == *output
                || result == output
                || call.inputs().len() != 2
                || call.inputs()[0] != *template.output()
                || call.inputs()[1].body() != &SchemaBody::String
                || !call.inputs()[1].dimension_parameters().is_empty()
                || !call.output().dimension_parameters().is_empty()
                || call.output().body()
                    != if call.primitive() == P::MapContainsKey {
                        &SchemaBody::Bool
                    } else {
                        &SchemaBody::UnsignedInteger(IntegerWidth::W64)
                    }
            {
                continue;
            }
            // No source-scope boundary or lifetime transition occurs between
            // these adjacent calls. The unchanged canonical root remains in
            // its ordinary arena; the runtime rechecks its exact generation.
            let id = out.regions.len();
            if out.projects[project].is_some() || out.reads[read].is_some() {
                continue;
            }
            out.regions.push(Region {
                owner,
                exit,
                source: source.get(),
                ordinal,
                output: output.get(),
                project,
                read,
                iteration: block.iteration(),
            });
            out.projects[project] = Some(id);
            out.reads[read] = Some(id);
        }
        Ok(out)
    }
}
