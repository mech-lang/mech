use std::collections::HashMap;

use mech_core::{MResult, MechError, MechErrorKind};

use crate::{
    SourceContextBase, SourceContextCapability, SourceContextCapabilityScope,
    SourceContextDeclaration, SourceScope,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeContextCapability {
    pub operation: String,
    pub scope: RuntimeContextCapabilityScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeContextCapabilityScope {
    Path(String),
    Wildcard,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeContextBase {
    ResourceUri(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeContextBinding {
    pub name: String,
    pub base: RuntimeContextBase,
    pub capabilities: Vec<RuntimeContextCapability>,
    pub scope: SourceScope,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeContextRegistry {
    bindings: HashMap<String, RuntimeContextBinding>,
}

impl RuntimeContextRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_declarations(
        scope: SourceScope,
        declarations: &[SourceContextDeclaration],
    ) -> MResult<Self> {
        let mut registry = Self::new();
        for declaration in declarations {
            let binding = RuntimeContextBinding::from_source(scope.clone(), declaration)?;
            registry.insert(binding)?;
        }
        Ok(registry)
    }

    pub fn insert(&mut self, binding: RuntimeContextBinding) -> MResult<()> {
        if self.bindings.contains_key(&binding.name) {
            return Err(MechError::new(
                RuntimeContextDuplicateBinding { name: binding.name },
                None,
            ));
        }
        self.bindings.insert(binding.name.clone(), binding);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&RuntimeContextBinding> {
        self.bindings.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl RuntimeContextBinding {
    pub fn from_source(
        scope: SourceScope,
        declaration: &SourceContextDeclaration,
    ) -> MResult<Self> {
        if declaration.name.is_empty() {
            return Err(MechError::new(
                RuntimeContextInvalidBinding {
                    name: declaration.name.clone(),
                    reason: "context name cannot be empty".to_string(),
                },
                None,
            ));
        }

        let base = match &declaration.base {
            SourceContextBase::ResourceUri(uri) => {
                if uri.is_empty() {
                    return Err(MechError::new(
                        RuntimeContextInvalidBinding {
                            name: declaration.name.clone(),
                            reason: "resource URI cannot be empty".to_string(),
                        },
                        None,
                    ));
                }
                RuntimeContextBase::ResourceUri(uri.clone())
            }
            SourceContextBase::Context(name) => {
                return Err(MechError::new(
                    RuntimeContextDerivedBaseUnsupported {
                        name: declaration.name.clone(),
                        base: name.clone(),
                    },
                    None,
                ));
            }
        };

        let mut capabilities = Vec::with_capacity(declaration.capabilities.len());
        for capability in &declaration.capabilities {
            capabilities.push(runtime_context_capability_from_source(
                &declaration.name,
                capability,
            )?);
        }

        Ok(Self {
            name: declaration.name.clone(),
            base,
            capabilities,
            scope,
        })
    }
}

fn runtime_context_capability_from_source(
    context_name: &str,
    capability: &SourceContextCapability,
) -> MResult<RuntimeContextCapability> {
    if capability.operation.is_empty() {
        return Err(MechError::new(
            RuntimeContextInvalidBinding {
                name: context_name.to_string(),
                reason: "capability operation cannot be empty".to_string(),
            },
            None,
        ));
    }

    let scope = match &capability.scope {
        SourceContextCapabilityScope::Path(path) => {
            RuntimeContextCapabilityScope::Path(path.clone())
        }
        SourceContextCapabilityScope::Wildcard => RuntimeContextCapabilityScope::Wildcard,
    };

    Ok(RuntimeContextCapability {
        operation: capability.operation.clone(),
        scope,
    })
}

#[derive(Debug, Clone)]
pub struct RuntimeContextDuplicateBinding {
    pub name: String,
}

impl MechErrorKind for RuntimeContextDuplicateBinding {
    fn name(&self) -> &str {
        "RuntimeContextDuplicateBinding"
    }

    fn message(&self) -> String {
        format!("runtime context `{}` is declared more than once", self.name)
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeContextInvalidBinding {
    pub name: String,
    pub reason: String,
}

impl MechErrorKind for RuntimeContextInvalidBinding {
    fn name(&self) -> &str {
        "RuntimeContextInvalidBinding"
    }

    fn message(&self) -> String {
        format!("invalid runtime context `{}`: {}", self.name, self.reason)
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeContextDerivedBaseUnsupported {
    pub name: String,
    pub base: String,
}

impl MechErrorKind for RuntimeContextDerivedBaseUnsupported {
    fn name(&self) -> &str {
        "RuntimeContextDerivedBaseUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "runtime context `{}` derives from `{}`, but derived context bases are not supported yet",
            self.name, self.base,
        )
    }
}
