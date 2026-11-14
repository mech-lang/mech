//! Artifact-owned descriptive lexical lookup. No selected factory, execution
//! permission, current-visit proof or admission result is cached here.
use super::*;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Clones of the same immutable artifact can share the index. Finalizing or
/// decoding another artifact starts with an empty cache, independently of its
/// revision. No revision-keyed/global lookup is involved.
#[derive(Clone, Debug, Default)]
pub(crate) struct NativeCollectionLexicalCache {
    owners: OnceLock<Arc<[OnceLock<NativeCollectionLexicalIndex>]>>,
}

#[derive(Debug, Default)]
struct NativeCollectionLexicalIndex {
    sites: BTreeMap<(u32, u32), Result<Site, Error>>,
    // Original traversal stops at its first metadata error. Sites encountered
    // earlier remain queryable; an unseen site receives that same error.
    terminal_error: Option<Error>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Site {
    operation: OperationReference,
    contract: OperationContractId,
    inputs: Box<[SchemaId]>,
    output: SchemaId,
    sources: Box<[NativeCollectionLexicalSource]>,
}

impl NativeCollectionLexicalCache {
    pub(super) fn select(
        &self,
        artifact: &ProgramArtifact,
        owner: NodeId,
        block: crate::ControlBlockId,
        local: u32,
        catalog: &FunctionCatalog,
    ) -> Result<NativeCollectionLexicalTemplate, Error> {
        let fail = || Error::InvalidBinding { node: Some(owner) };
        let root = artifact
            .nodes()
            .get(owner.get() as usize)
            .filter(|root| root.node == owner)
            .ok_or_else(fail)?;
        let owners = self.owners.get_or_init(|| {
            (0..artifact.nodes().len())
                .map(|_| OnceLock::new())
                .collect::<Vec<_>>()
                .into()
        });
        let index = owners
            .get(owner.get() as usize)
            .ok_or_else(fail)?
            .get_or_init(|| NativeCollectionLexicalIndex::build(artifact, owner, &root.body));
        let site = index.site(owner, block, local)?;
        // Selection is deliberately performed on every call against the actual
        // catalog, contracts and schemas. The cache only replaces tree scans.
        let leaf = select_leaf(
            artifact,
            owner,
            &site.operation,
            site.contract,
            &site.inputs,
            site.output,
            catalog,
        )?;
        Ok(NativeCollectionLexicalTemplate {
            revision: artifact.revision(),
            owner,
            block,
            local,
            sources: site.sources.clone(),
            leaf,
        })
    }
}

impl NativeCollectionLexicalIndex {
    fn build(artifact: &ProgramArtifact, owner: NodeId, body: &ExecutableNodeBody) -> Self {
        let mut index = Self::default();
        index.terminal_error = (|| {
            let mut pending = Vec::new();
            match body {
                ExecutableNodeBody::Match(declaration)
                | ExecutableNodeBody::Activation(declaration) => {
                    push_match(declaration, &mut pending)
                }
                ExecutableNodeBody::Iteration(declaration) => {
                    pending.push(LexicalScope::Block(&declaration.body))
                }
                ExecutableNodeBody::Comprehension(declaration) => {
                    let inputs = node_inputs(artifact, owner)?
                        .iter()
                        .map(|source| source_schema_id(artifact, *source))
                        .collect::<Result<Vec<_>, _>>()?;
                    pending.push(LexicalScope::Comprehension(declaration, inputs));
                }
                _ => {
                    return Err(unsupported(
                        owner,
                        "root has no lexical collection declaration tree",
                    ));
                }
            }
            index.collect(artifact, owner, pending)
        })()
        .err();
        index
    }

    fn site(
        &self,
        owner: NodeId,
        block: crate::ControlBlockId,
        local: u32,
    ) -> Result<&Site, Error> {
        match self.sites.get(&(block.0, local)) {
            Some(Ok(site)) => Ok(site),
            Some(Err(error)) => Err(error.clone()),
            None => Err(self
                .terminal_error
                .clone()
                .unwrap_or(Error::InvalidBinding { node: Some(owner) })),
        }
    }

    /// Return inputs only for a body that may introduce a child scope. Pure
    /// sites move their schema vector into the index, without a second copy.
    fn record(
        &mut self,
        owner: NodeId,
        block: crate::ControlBlockId,
        local: u32,
        inputs: Vec<SchemaId>,
        output: SchemaId,
        body: &crate::ControlOperationBody,
        sources: impl FnOnce() -> Box<[NativeCollectionLexicalSource]>,
    ) -> Option<Vec<SchemaId>> {
        match body {
            crate::ControlOperationBody::Operation {
                operation,
                contract,
            } => {
                // The old lookup returns the first matching declaration, even
                // if a malformed tree repeats a block/local identity.
                self.sites.entry((block.0, local)).or_insert_with(|| {
                    Ok(Site {
                        operation: operation.clone(),
                        contract: *contract,
                        inputs: inputs.into_boxed_slice(),
                        output,
                        sources: sources(),
                    })
                });
                None
            }
            _ => {
                self.sites.entry((block.0, local)).or_insert_with(|| {
                    Err(unsupported(
                        owner,
                        "lexical collection site is not a pure library operation",
                    ))
                });
                Some(inputs)
            }
        }
    }

    fn collect<'a>(
        &mut self,
        artifact: &ProgramArtifact,
        owner: NodeId,
        mut pending: Vec<LexicalScope<'a>>,
    ) -> Result<(), Error> {
        while let Some(scope) = pending.pop() {
            match scope {
                LexicalScope::Block(block) => {
                    for operation in &block.operations {
                        let inputs = operation
                            .inputs
                            .iter()
                            .map(|value| {
                                control_source_schema(
                                    artifact,
                                    owner,
                                    block,
                                    operation.node,
                                    *value,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        let child_inputs = self.record(
                            owner,
                            block.id,
                            operation.node,
                            inputs,
                            operation.schema,
                            &operation.body,
                            || {
                                operation
                                    .inputs
                                    .iter()
                                    .copied()
                                    .map(NativeCollectionLexicalSource::Control)
                                    .collect()
                            },
                        );
                        if let Some(inputs) = child_inputs {
                            push_children(&operation.body, inputs, &mut pending);
                        }
                    }
                }
                LexicalScope::Comprehension(declaration, inputs) => {
                    let mut locals = BTreeMap::new();
                    for step in &declaration.steps {
                        match step {
                            crate::ComprehensionStep::Generator { pattern, .. } => {
                                bind_pattern_schemas(owner, pattern, &mut locals)?
                            }
                            crate::ComprehensionStep::Operation(operation) => {
                                let actual_inputs = operation
                                    .inputs
                                    .iter()
                                    .map(|value| {
                                        comprehension_source_schema(
                                            artifact, owner, &inputs, &locals, *value,
                                        )
                                    })
                                    .collect::<Result<Vec<_>, _>>()?;
                                let child_inputs = self.record(
                                    owner,
                                    declaration.id,
                                    operation.local,
                                    actual_inputs,
                                    operation.schema,
                                    &operation.body,
                                    || {
                                        operation
                                            .inputs
                                            .iter()
                                            .copied()
                                            .map(NativeCollectionLexicalSource::Comprehension)
                                            .collect()
                                    },
                                );
                                // A lookup at this operation originally returns
                                // before inserting its local. Record it first.
                                if locals.insert(operation.local, operation.schema).is_some() {
                                    return Err(Error::InvalidBinding { node: Some(owner) });
                                }
                                if let Some(inputs) = child_inputs {
                                    push_children(&operation.body, inputs, &mut pending);
                                }
                            }
                            crate::ComprehensionStep::Filter(_) => {}
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "native_collection_lexical_index_tests.rs"]
mod tests;
