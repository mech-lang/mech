//! A selected matrix cell may lend its String only to one adjacent retained
//! insertion. This is a complete use proof, not a value/schema-based guess.
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as P, NativeIterationAction as A, NativeIterationTerminator as T,
    NativeIterationValueScope as Scope, NativeMapOwnerWords, NativeMatchTerminator as S,
    NativeMatchValue, NativeProgramArtifactPlan as Plan, NativeProgramReadRole as Read,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Region {
    pub owner: usize,
    pub producer: usize,
    pub consumer: usize,
    pub source: NativeMatchValue,
    pub selector: NativeMatchValue,
    pub output: NativeMatchValue,
}

/// A pure derived cache: independently reconstructed from the validated plan
/// on image load. No image-supplied String pointer or loan can be accepted.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
}
#[cfg(test)]
thread_local! {
    static PLAN_VALIDATIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_plan_validations() -> u64 {
    PLAN_VALIDATIONS.with(|v| v.replace(0))
}
impl Layout {
    pub fn producer(&self, site: usize) -> Option<(usize, &Region)> {
        self.regions
            .iter()
            .enumerate()
            .find(|(_, r)| r.producer == site)
    }
    pub fn consumer(&self, site: usize) -> Option<(usize, &Region)> {
        self.regions
            .iter()
            .enumerate()
            .find(|(_, r)| r.consumer == site)
    }
    pub fn derive(plan: &Plan, owners: Option<&NativeMapOwnerWords>) -> Result<Self, &'static str> {
        if owners.is_none_or(|o| o.regions().is_empty()) {
            return Ok(Self::default());
        }
        #[cfg(test)]
        PLAN_VALIDATIONS.with(|v| v.set(v.get() + 1));
        plan.validate_current_visits()?;
        Self::derive_validated(plan, owners)
    }

    /// Recompute all key-use facts for a genuine checked Object without
    /// repeating whole-program validity checks. No cached key pointer or
    /// image-supplied proof is accepted by this route.
    ///
    /// # Safety
    /// The caller must hold the private same-Object image witness establishing
    /// that this exact immutable plan was validated by the checked compiler.
    /// Independent/untrusted reconstruction must use `derive` instead.
    pub unsafe fn from_checked_object_plan(
        plan: &Plan,
        owners: Option<&NativeMapOwnerWords>,
    ) -> Result<Self, &'static str> {
        Self::derive_validated(plan, owners)
    }

    fn derive_validated(
        plan: &Plan,
        owners: Option<&NativeMapOwnerWords>,
    ) -> Result<Self, &'static str> {
        let Some(owners) = owners.filter(|o| !o.regions().is_empty()) else {
            return Ok(Self::default());
        };
        if plan.blocks().len() > 16_384 || plan.values().len() > 65_536 {
            return Ok(Self::default());
        }
        let facts = plan.facts();
        let mut out = Self::default();
        for (producer, block) in plan.blocks().iter().enumerate() {
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
            if template.primitive() != P::MatrixScalarAccess
                || operands.len() != 2
                || template.inputs().len() != 2
                || template.output().body() != &SchemaBody::String
                || !template.output().dimension_parameters().is_empty()
                || template.inputs()[1].body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
                || !template.inputs()[1].dimension_parameters().is_empty()
                || !matches!(template.inputs()[0].body(), SchemaBody::Matrix { element, dimensions }
                    if element.as_ref() == &SchemaBody::String && dimensions.len() == 2)
            {
                continue;
            }
            let [Read::Action(consumer)] = facts.read_roles(*output)? else {
                continue;
            };
            let consumer = *consumer as usize;
            let Some(owner) = owners.call_region(consumer) else {
                continue;
            };
            let owned = &owners.regions()[owner];
            let Some(iteration) = block.iteration() else {
                continue;
            };
            let Scope::Immutable {
                iteration: source_scope,
            } = plan.value_scopes()[operands[0].get() as usize]
            else {
                continue;
            };
            let [definition] = facts.definition_sites(output.get())? else {
                continue;
            };
            if owned.iteration() != iteration
                || source_scope == Some(iteration)
                || !facts.lexical_ancestor(source_scope, Some(iteration))
                || definition.site() != Some(producer as u32)
                || block.terminator() != T::Scalar(S::Jump(consumer as u32))
                || facts.predecessors(consumer as u32)? != [producer as u32]
                || plan.blocks()[consumer].iteration() != Some(iteration)
                || plan.activation_boundary() == Some(producer as u32)
                || plan.activation_boundary() == Some(consumer as u32)
            {
                continue;
            }
            let A::LibraryCall {
                template: insert,
                operands: inputs,
                output: result,
            } = plan.blocks()[consumer].action()
            else {
                continue;
            };
            if insert.primitive() != P::MapInsert
                || inputs.len() != 3
                || insert.inputs().len() != 3
                || inputs[1] != *output
                || inputs[0] == *output
                || inputs[2] == *output
                || result == output
                || insert.inputs()[1] != *template.output()
                || !owned.writes().contains(&(consumer as u32))
                || !matches!(insert.inputs()[0].body(), SchemaBody::Map { key, value, .. }
                    if key.as_ref() == &SchemaBody::String
                    && value.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64))
            {
                continue;
            }
            out.regions.push(Region {
                owner,
                producer,
                consumer,
                source: operands[0],
                selector: operands[1],
                output: *output,
            });
        }
        Ok(out)
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) mod tests;
