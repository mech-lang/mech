//! Metadata-only native lowering of exact synchronous tail transitions and
//! schema-directed structural observations. No source or resident evaluator.
use super::native_match_lowering::{Builder, Error, unsupported};
use crate::{
    CollectionPattern, ControlBlock, ControlOperationBody, ControlValue, MatchDeclaration,
    native_artifact::{
        NativeControlLoop, NativeIterationAction, NativeIterationTerminator,
        NativeIterationValueScope, NativeMatchAction, NativeMatchOrigin, NativeMatchTerminator,
        NativeMatchValue, NativeStructuralObservation, NativeStructuralStep, NativeTailLoop,
    },
};
use mech_core::{IntegerWidth, SchemaBody};
use std::collections::BTreeMap;

type ControlPattern = CollectionPattern<mech_core::SchemaId, crate::MatchPatternValue>;

pub(super) fn has_direct_recur(declaration: &MatchDeclaration) -> bool {
    declaration.arms.iter().any(|arm| {
        arm.body
            .operations
            .iter()
            .any(|op| matches!(op.body, ControlOperationBody::Recur(_)))
    })
}

/// Only the exact last operation and exact selected local result may be
/// replaced by a backedge. An unused/observable operation after recursion, a
/// different yield, nonzero lexical ancestor or nested Recur stays unsupported.
/// This prefix is a compiler view of unchanged canonical operation identities;
/// leaf selection still rederives each site from the original artifact.
pub(super) fn tail_prefix(block: &ControlBlock) -> Result<Option<ControlBlock>, Error> {
    let Some(last) = block.operations.last() else {
        return Ok(None);
    };
    let ControlOperationBody::Recur(ancestor) = &last.body else {
        return Ok(None);
    };
    if *ancestor != 0
        || last.inputs.len() != 1
        || block.yield_value
            != (ControlValue::Local {
                block: block.id,
                node: last.node,
            })
        || block.operations[..block.operations.len() - 1]
            .iter()
            .any(|op| matches!(op.body, ControlOperationBody::Recur(_)))
    {
        return Err(Error::UnsupportedProgram {
            feature: "non-tail or foreign native Recur",
        });
    }
    Ok(Some(ControlBlock {
        id: block.id,
        parameters: block.parameters.clone(),
        operations: block.operations[..block.operations.len() - 1].into(),
        yield_value: last.inputs[0],
    }))
}

impl Builder<'_> {
    fn native_bool_value(&mut self, owner: mech_core::NodeId) -> Result<NativeMatchValue, Error> {
        let id = self
            .artifact
            .schemas()
            .entries()
            .enumerate()
            .find(|(_, entry)| {
                entry.schema().body() == &SchemaBody::Bool
                    && entry.schema().dimension_parameters().is_empty()
            })
            .and_then(|(index, _)| u32::try_from(index).ok())
            .map(mech_core::SchemaId::new)
            .ok_or(Error::InvalidBinding { node: Some(owner) })?;
        self.value(self.type_for_schema(id)?)
    }

    pub(super) fn managed_literal_branch(
        &mut self,
        at: NativeMatchOrigin,
        source: NativeMatchValue,
        peer: NativeMatchValue,
        entry: u32,
        yes: u32,
        no: u32,
    ) -> Result<(), Error> {
        if self.ty(source) != self.ty(peer)
            || self.ty(source).definition().body() != &SchemaBody::String
            || !self
                .ty(source)
                .definition()
                .dimension_parameters()
                .is_empty()
        {
            return Err(unsupported(at.node(), "non-String managed literal pattern"));
        }
        let condition = self.native_bool_value(at.node())?;
        let test = self.block(at, NativeMatchAction::None)?;
        self.blocks[test as usize].action = NativeIterationAction::Structural {
            source,
            path: Box::new([]),
            observation: NativeStructuralObservation::StringEqual { peer },
            output: condition,
        };
        self.blocks[test as usize].terminator =
            NativeMatchTerminator::Branch { condition, yes, no }.into();
        self.link(entry, test);
        Ok(())
    }

    pub(super) fn tail_match(
        &mut self,
        at: NativeMatchOrigin,
        declaration: &MatchDeclaration,
        arguments: &[NativeMatchValue],
        output: NativeMatchValue,
    ) -> Result<(u32, u32), Error> {
        let owner = at.node();
        if self.loop_depth >= crate::MAX_CONTROL_DEPTH {
            return Err(unsupported(owner, "native tail-loop depth"));
        }
        let seed = *arguments
            .get(declaration.scrutinee as usize)
            .ok_or(Error::InvalidBinding { node: Some(owner) })?;
        // This first closed synchronous lowering carries nominal enum state.
        // General non-tail recursion, suspension and publication remain closed.
        let SchemaBody::Enum { variants, .. } = self.ty(seed).definition().body() else {
            return Err(unsupported(owner, "non-enum native tail state"));
        };
        // A sticky/partial Match is not silently turned into a stateless
        // failure. The closed subset proves every exact enum state has an
        // unguarded irrefutable payload arm, independently of source notes.
        fn irrefutable(pattern: &ControlPattern) -> bool {
            match pattern {
                CollectionPattern::Wildcard | CollectionPattern::Bind { .. } => true,
                CollectionPattern::Tuple(items) => items.iter().all(irrefutable),
                _ => false,
            }
        }
        if (0..variants.len()).any(|ordinal| !declaration.arms.iter().any(|arm| {
            arm.guard.is_none() && matches!(&arm.pattern,
                crate::MatchPattern::Structural(CollectionPattern::Enum { ordinal: actual, payload })
                    if *actual as usize == ordinal
                        && payload.as_deref().is_none_or(irrefutable))
        })) {
            return Err(unsupported(owner, "native tail Match is not structurally total"));
        }
        for arm in &declaration.arms {
            let _ = tail_prefix(&arm.body)?;
        }
        let id = u32::try_from(self.iterations.len())
            .map_err(|_| Error::InvalidBinding { node: Some(owner) })?;
        let parent = self.current_iteration;
        self.iterations.push(None);
        let initialize = self.block(at, NativeMatchAction::None)?;
        self.blocks[initialize as usize].action =
            NativeIterationAction::Initialize { iteration: id };
        self.current_iteration = Some(id);
        self.loop_depth += 1;
        let lowered: Result<_, Error> = (|| {
            let carrier = self.value(self.ty(seed).clone())?;
            self.value_scopes[carrier.get() as usize] =
                NativeIterationValueScope::Carrier { iteration: id };
            let next = self.value(self.ty(seed).clone())?;
            let returned = self.value(self.ty(output).clone())?;
            let header = self.block(at, NativeMatchAction::None)?;
            let latch = self.block(at, NativeMatchAction::None)?;
            self.blocks[latch as usize].action = NativeIterationAction::Latch { iteration: id };
            let mut args = arguments.to_vec();
            args[declaration.scrutinee as usize] = carrier;
            let (body, finish) =
                self.matched_body(at, declaration, &args, returned, Some((next, latch)))?;
            self.blocks[header as usize].terminator = NativeIterationTerminator::TailHeader {
                iteration: id,
                body,
            };
            self.link(latch, header);
            Ok((carrier, next, returned, header, body, latch, finish))
        })();
        self.loop_depth -= 1;
        self.current_iteration = parent;
        let (carrier, next, returned, header, body, latch, finish) = lowered?;
        let exit = self.block(
            at,
            NativeMatchAction::Copy {
                source: returned,
                output,
            },
        )?;
        self.link(finish, exit);
        self.link(initialize, header);
        self.iterations[id as usize] = Some(NativeControlLoop::Tail(NativeTailLoop {
            origin: at,
            parent,
            seed,
            carrier,
            next,
            returned,
            result: output,
            initialize,
            header,
            body,
            latch,
            finish,
            exit,
        }));
        Ok((initialize, exit))
    }

    /// Return a success block, with every failed tag test branching directly
    /// to the next ordered arm. Projection runs only after its owning tag test.
    pub(super) fn structural_pattern(
        &mut self,
        at: NativeMatchOrigin,
        pattern: &ControlPattern,
        source: NativeMatchValue,
        entry: u32,
        failed: u32,
        bindings: &mut BTreeMap<u32, NativeMatchValue>,
    ) -> Result<u32, Error> {
        let body = self.ty(source).definition().body().clone();
        self.structural_walk(
            at,
            pattern,
            source,
            &body,
            &mut Vec::new(),
            entry,
            failed,
            bindings,
            0,
        )
    }

    fn structural_walk(
        &mut self,
        at: NativeMatchOrigin,
        pattern: &ControlPattern,
        source: NativeMatchValue,
        body: &SchemaBody,
        path: &mut Vec<NativeStructuralStep>,
        mut tail: u32,
        failed: u32,
        bindings: &mut BTreeMap<u32, NativeMatchValue>,
        depth: usize,
    ) -> Result<u32, Error> {
        let owner = at.node();
        if depth > crate::MAX_CONTROL_DEPTH {
            return Err(unsupported(owner, "native structural pattern depth"));
        }
        match pattern {
            CollectionPattern::Wildcard => {}
            CollectionPattern::Bind { local, schema } => {
                let ty = self.type_for_schema(*schema)?;
                if ty.definition().body() != body
                    || !ty.definition().dimension_parameters().is_empty()
                {
                    return Err(unsupported(owner, "incompatible structural native binding"));
                }
                if *body != SchemaBody::UnsignedInteger(IntegerWidth::W64)
                    && !crate::native_artifact::structural_values::supported(body)
                {
                    return Err(unsupported(
                        owner,
                        "structural child requires a native draft expansion witness",
                    ));
                }
                let output = self.value(ty)?;
                if bindings.insert(*local, output).is_some() {
                    return Err(Error::InvalidBinding { node: Some(owner) });
                }
                let block = self.block(at, NativeMatchAction::None)?;
                self.blocks[block as usize].action = NativeIterationAction::Structural {
                    source,
                    path: path.clone().into(),
                    observation: if *body == SchemaBody::UnsignedInteger(IntegerWidth::W64) {
                        NativeStructuralObservation::U64
                    } else {
                        NativeStructuralObservation::CanonicalValue
                    },
                    output,
                };
                self.link(tail, block);
                tail = block;
            }
            CollectionPattern::Tuple(items) => {
                let SchemaBody::Tuple(fields) = body else {
                    return Err(Error::InvalidBinding { node: Some(owner) });
                };
                if fields.len() != items.len() {
                    return Err(Error::InvalidBinding { node: Some(owner) });
                }
                for (ordinal, (item, field)) in items.iter().zip(fields).enumerate() {
                    path.push(NativeStructuralStep::TupleItem {
                        ordinal: u32::try_from(ordinal)
                            .map_err(|_| Error::InvalidBinding { node: Some(owner) })?,
                    });
                    tail = self.structural_walk(
                        at,
                        item,
                        source,
                        field,
                        path,
                        tail,
                        failed,
                        bindings,
                        depth + 1,
                    )?;
                    path.pop();
                }
            }
            CollectionPattern::Enum { ordinal, payload } => {
                let SchemaBody::Enum { variants, .. } = body else {
                    return Err(Error::InvalidBinding { node: Some(owner) });
                };
                let variant = variants
                    .get(*ordinal as usize)
                    .ok_or(Error::InvalidBinding { node: Some(owner) })?;
                let condition = self.native_bool_value(owner)?;
                let test = self.block(at, NativeMatchAction::None)?;
                self.blocks[test as usize].action = NativeIterationAction::Structural {
                    source,
                    path: path.clone().into(),
                    observation: NativeStructuralObservation::EnumOrdinal { ordinal: *ordinal },
                    output: condition,
                };
                self.link(tail, test);
                let success = self.block(at, NativeMatchAction::None)?;
                self.blocks[test as usize].terminator = NativeMatchTerminator::Branch {
                    condition,
                    yes: success,
                    no: failed,
                }
                .into();
                tail = success;
                match (payload.as_deref(), variant.payload.as_ref()) {
                    (None, None) => {}
                    (Some(pattern), Some(expected)) => {
                        path.push(NativeStructuralStep::EnumPayload { ordinal: *ordinal });
                        tail = self.structural_walk(
                            at,
                            pattern,
                            source,
                            expected,
                            path,
                            tail,
                            failed,
                            bindings,
                            depth + 1,
                        )?;
                        path.pop();
                    }
                    _ => return Err(Error::InvalidBinding { node: Some(owner) }),
                }
            }
            _ => {
                return Err(unsupported(
                    owner,
                    "native structural equality or array pattern",
                ));
            }
        }
        Ok(tail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ControlBlockId, ControlOperation};
    use mech_core::{ConstantId, SchemaId};

    fn terminal() -> ControlBlock {
        let id = ControlBlockId(3);
        ControlBlock {
            id,
            parameters: Box::new([]),
            operations: vec![ControlOperation {
                node: 0,
                body: ControlOperationBody::Recur(0),
                inputs: vec![ControlValue::Constant(ConstantId::new(0))].into(),
                schema: SchemaId::new(1),
            }]
            .into(),
            yield_value: ControlValue::Local { block: id, node: 0 },
        }
    }

    #[test]
    fn exact_final_recur_prefix_preserves_the_real_canonical_argument() {
        let block = terminal();
        let prefix = tail_prefix(&block).unwrap().unwrap();
        assert_eq!(prefix.id, block.id);
        assert!(prefix.operations.is_empty());
        assert_eq!(prefix.yield_value, block.operations[0].inputs[0]);
        assert_eq!(block.operations.len(), 1);
    }

    #[test]
    fn recur_cannot_discard_a_different_selected_result_or_call_a_foreign_ancestor() {
        let mut wrong_yield = terminal();
        wrong_yield.yield_value = ControlValue::Constant(ConstantId::new(0));
        assert!(tail_prefix(&wrong_yield).is_err());
        let mut foreign = terminal();
        foreign.operations[0].body = ControlOperationBody::Recur(1);
        assert!(tail_prefix(&foreign).is_err());
        let mut malformed = terminal();
        malformed.operations[0].inputs = Box::new([]);
        assert!(tail_prefix(&malformed).is_err());
    }

    #[test]
    fn a_previous_recursive_call_is_not_swallowed_by_a_later_tail_call() {
        let mut block = terminal();
        let mut last = block.operations[0].clone();
        last.node = 1;
        block.operations = vec![block.operations[0].clone(), last].into();
        block.yield_value = ControlValue::Local {
            block: block.id,
            node: 1,
        };
        assert!(tail_prefix(&block).is_err());
    }
}
