//! Canonical qualifiers -> generated structured loops and staged yields.
//! No collection, body operation, or source expression executes while lowering.
use super::native_match_lowering::Builder;
use crate::native_artifact::{
    NativeArtifactLoweringError as Error, NativeCollectionLoop, NativeControlLoop,
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar, NativeMatchOrigin as Origin, NativeMatchTerminator as Term,
    NativeMatchValue as Value, NativeProgramCollection,
};
use crate::{
    CollectionPattern, ComprehensionDeclaration, ComprehensionKind, ComprehensionStep,
    ComprehensionValue, ControlOperationBody,
};
use mech_core::{FloatWidth, IntegerWidth, SchemaBody};
use std::collections::BTreeMap;

fn unsupported(at: Origin, feature: &'static str) -> Error {
    Error::UnsupportedNode {
        node: at.node(),
        feature,
    }
}
fn invalid(at: Origin) -> Error {
    Error::InvalidBinding {
        node: Some(at.node()),
    }
}
fn at(owner: Origin, block: u32, local: Option<u32>) -> Origin {
    Origin {
        node: owner.node(),
        block: Some(block),
        local,
    }
}
pub(crate) fn element_supported(body: &SchemaBody) -> bool {
    matches!(
        body,
        SchemaBody::Bool
            | SchemaBody::Index
            | SchemaBody::String
            | SchemaBody::UnsignedInteger(IntegerWidth::W64)
            | SchemaBody::SignedInteger(IntegerWidth::W64)
            | SchemaBody::FloatingPoint(FloatWidth::W64)
    )
}

#[derive(Clone, Copy)]
struct QualifierContext<'a> {
    owner: Origin,
    declaration: &'a ComprehensionDeclaration,
    arguments: &'a [Value],
    collection: u32,
}

impl Builder<'_> {
    fn collection_read(
        &self,
        value: ComprehensionValue,
        arguments: &[Value],
        locals: &BTreeMap<u32, Value>,
        owner: Origin,
    ) -> Result<Value, Error> {
        match value {
            ComprehensionValue::Input(index) => arguments.get(index as usize).copied(),
            ComprehensionValue::Local(index) => locals.get(&index).copied(),
            ComprehensionValue::Constant(id) => self.constants.get(&id).copied(),
        }
        .ok_or_else(|| invalid(owner))
    }

    pub(super) fn comprehension(
        &mut self,
        owner: Origin,
        declaration: &ComprehensionDeclaration,
        arguments: &[Value],
        output: Value,
    ) -> Result<(u32, u32), Error> {
        if !self.allow_collections
            || !matches!(
                declaration.kind,
                ComprehensionKind::Matrix | ComprehensionKind::MatrixPreserveShape
            )
        {
            return Err(unsupported(owner, "native collection construction kind"));
        }
        let SchemaBody::Matrix { element, .. } = self.ty(output).definition().body() else {
            return Err(invalid(owner));
        };
        if !element_supported(element) {
            return Err(unsupported(owner, "native collected element kind"));
        }
        let collection = u32::try_from(self.collections.len()).map_err(|_| invalid(owner))?;
        self.collections.push(None);
        let parent = self.current_iteration;
        let parent_collection = self.current_collection.replace(collection);
        let begin = self.block(owner, Scalar::None)?;
        self.blocks[begin as usize].action = Action::CollectionBegin { collection };
        let finish = self.block(owner, Scalar::None)?;
        self.blocks[finish as usize].action = Action::CollectionFinish { collection };
        let mut generators = Vec::new();
        let body = self.collection_steps(
            &QualifierContext {
                owner,
                declaration,
                arguments,
                collection,
            },
            0,
            BTreeMap::new(),
            &mut generators,
            finish,
        );
        self.current_collection = parent_collection;
        let (entry, tail, push) = body?;
        self.link(begin, entry);
        self.link(tail, finish);
        if declaration.kind == ComprehensionKind::MatrixPreserveShape && generators.len() != 1 {
            return Err(invalid(owner));
        }
        self.collections[collection as usize] = Some(NativeProgramCollection {
            origin: owner,
            parent,
            parent_collection,
            kind: declaration.kind,
            output,
            generators: generators.into(),
            begin,
            push,
            finish,
        });
        Ok((begin, finish))
    }

    fn collection_steps(
        &mut self,
        context: &QualifierContext<'_>,
        start: usize,
        mut locals: BTreeMap<u32, Value>,
        generators: &mut Vec<u32>,
        skip: u32,
    ) -> Result<(u32, u32, u32), Error> {
        let QualifierContext {
            owner,
            declaration,
            arguments,
            collection,
        } = *context;
        let site = at(owner, declaration.id.0, None);
        let entry = self.block(site, Scalar::None)?;
        let mut tail = entry;
        for (offset, step) in declaration.steps.iter().enumerate().skip(start) {
            match step {
                ComprehensionStep::Generator { source, pattern } => {
                    if self.loop_depth > crate::MAX_CONTROL_DEPTH {
                        return Err(unsupported(
                            owner,
                            "native loop nesting exceeds control depth",
                        ));
                    }
                    let source = self.collection_read(*source, arguments, &locals, owner)?;
                    let id = u32::try_from(self.iterations.len()).map_err(|_| invalid(owner))?;
                    self.iterations.push(None);
                    generators.push(id);
                    let parent = self.current_iteration;
                    let initialize = self.block(site, Scalar::None)?;
                    self.blocks[initialize as usize].action = Action::Initialize { iteration: id };
                    self.link(tail, initialize);
                    self.current_iteration = Some(id);
                    self.loop_depth += 1;
                    let nested = (|| {
                        let element = match pattern {
                            CollectionPattern::Wildcard => None,
                            CollectionPattern::Bind { local, schema } => {
                                let ty = self.type_for_schema(*schema)?;
                                if !element_supported(ty.definition().body())
                                    || !ty.definition().dimension_parameters().is_empty()
                                {
                                    return Err(unsupported(
                                        owner,
                                        "native generator binding kind",
                                    ));
                                }
                                let value = self.value(ty)?;
                                if locals.insert(*local, value).is_some() {
                                    return Err(invalid(owner));
                                }
                                Some(value)
                            }
                            _ => {
                                return Err(unsupported(
                                    owner,
                                    "native structural generator pattern",
                                ));
                            }
                        };
                        let header = self.block(site, Scalar::None)?;
                        let body = self.block(site, Scalar::None)?;
                        self.blocks[body as usize].action =
                            Action::CollectionElement { iteration: id };
                        let latch = self.block(site, Scalar::None)?;
                        self.blocks[latch as usize].action = Action::Latch { iteration: id };
                        let (rest, end, push) =
                            self.collection_steps(context, offset + 1, locals, generators, latch)?;
                        self.link(body, rest);
                        self.link(end, latch);
                        self.link(latch, header);
                        Ok((element, header, body, latch, push))
                    })();
                    self.loop_depth -= 1;
                    self.current_iteration = parent;
                    let (element, header, body, latch, push) = nested?;
                    let exit = self.block(site, Scalar::None)?;
                    self.blocks[header as usize].terminator = Terminator::Header {
                        iteration: id,
                        body,
                        exit,
                    };
                    self.link(initialize, header);
                    self.iterations[id as usize] =
                        Some(NativeControlLoop::Collection(NativeCollectionLoop {
                            origin: site,
                            parent,
                            collection,
                            source,
                            element,
                            initialize,
                            header,
                            body,
                            latch,
                            exit,
                        }));
                    return Ok((entry, exit, push));
                }
                ComprehensionStep::Filter(value) => {
                    let condition = self.collection_read(*value, arguments, &locals, owner)?;
                    if self.ty(condition).scalar_kind()
                        != Some(crate::native_artifact::NativeScalarKind::Bool)
                    {
                        return Err(invalid(owner));
                    }
                    let next = self.block(site, Scalar::None)?;
                    self.blocks[tail as usize].terminator = Term::Branch {
                        condition,
                        yes: next,
                        no: skip,
                    }
                    .into();
                    tail = next;
                }
                ComprehensionStep::Operation(operation) => {
                    let args = operation
                        .inputs
                        .iter()
                        .map(|value| self.collection_read(*value, arguments, &locals, owner))
                        .collect::<Result<Vec<_>, _>>()?;
                    let result = self.value(self.type_for_schema(operation.schema)?)?;
                    let operation_site = at(owner, declaration.id.0, Some(operation.local));
                    let (first, last) = match &operation.body {
                        ControlOperationBody::Operation {
                            operation,
                            contract,
                        } => {
                            let block = self.primitive(
                                operation_site,
                                operation,
                                *contract,
                                &args,
                                result,
                            )?;
                            (block, block)
                        }
                        ControlOperationBody::Match(control) => {
                            self.matched(operation_site, control, &args, result)?
                        }
                        ControlOperationBody::Iteration(control) => {
                            self.iteration(operation_site, control, &args, result)?
                        }
                        ControlOperationBody::Comprehension(control) => {
                            self.comprehension(operation_site, control, &args, result)?
                        }
                        _ => return Err(unsupported(owner, "native collection body lifecycle")),
                    };
                    self.link(tail, first);
                    tail = last;
                    if locals.insert(operation.local, result).is_some() {
                        return Err(invalid(owner));
                    }
                }
            }
        }
        let value = self.collection_read(declaration.yield_value, arguments, &locals, owner)?;
        if !element_supported(self.ty(value).definition().body())
            || !self
                .ty(value)
                .definition()
                .dimension_parameters()
                .is_empty()
        {
            return Err(unsupported(owner, "native collected yield kind"));
        }
        let push = self.block(site, Scalar::None)?;
        self.blocks[push as usize].action = Action::CollectionPush { collection, value };
        self.link(tail, push);
        Ok((entry, push, push))
    }
}
