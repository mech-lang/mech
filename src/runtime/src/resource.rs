use std::collections::HashMap;

use mech_core::{MResult, MechError, MechErrorKind, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResourceReadRequest {
    pub base_uri: String,
    pub path: String,
    pub context_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeResourceWriteIntent {
    Assign,
    Send,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResourceWritePreflightRequest {
    pub base_uri: String,
    pub path: String,
    pub context_name: String,
    pub intent: RuntimeResourceWriteIntent,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeResourceWriteRequest {
    pub base_uri: String,
    pub path: String,
    pub context_name: String,
    pub value: Value,
    pub intent: RuntimeResourceWriteIntent,
}

pub trait RuntimeResourceProvider: std::fmt::Debug {
    fn scheme(&self) -> &str;

    fn base_uris(&self) -> Vec<String> {
        Vec::new()
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value>;

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        Err(MechError::new(
            RuntimeResourceWriteUnsupported {
                scheme: self.scheme().to_string(),
                base_uri: request.base_uri,
                path: request.path,
            },
            None,
        ))
    }

    fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        Err(MechError::new(
            RuntimeResourceWriteUnsupported {
                scheme: self.scheme().to_string(),
                base_uri: request.base_uri,
                path: request.path,
            },
            None,
        ))
    }
}

#[derive(Debug, Default)]
pub struct RuntimeResourceRegistry {
    providers: Vec<Box<dyn RuntimeResourceProvider>>,
}

impl RuntimeResourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register_provider(&mut self, provider: Box<dyn RuntimeResourceProvider>) -> MResult<()> {
        let scheme = provider.scheme().to_string();
        if scheme.is_empty() {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: String::new(),
                    reason: "resource provider scheme cannot be empty".to_string(),
                },
                None,
            ));
        }
        let bases = provider.base_uris();
        if bases.is_empty()
            && self
                .providers
                .iter()
                .any(|p| p.scheme() == scheme && p.base_uris().is_empty())
        {
            return Err(MechError::new(
                RuntimeResourceProviderConflict { scheme },
                None,
            ));
        }
        for base in &bases {
            if resource_uri_scheme(base)? != scheme {
                return Err(MechError::new(
                    RuntimeResourceInvalidUri {
                        uri: base.clone(),
                        reason: format!("provider base URI scheme must be `{scheme}`"),
                    },
                    None,
                ));
            }
            for existing in &self.providers {
                if existing.scheme() == scheme
                    && existing.base_uris().iter().any(|other| other == base)
                {
                    return Err(MechError::new(
                        RuntimeResourceProviderConflict {
                            scheme: scheme.clone(),
                        },
                        None,
                    ));
                }
            }
        }
        self.providers.push(provider);
        Ok(())
    }
    pub fn has_provider(&self, scheme: &str) -> bool {
        self.providers.iter().any(|p| p.scheme() == scheme)
    }
    pub fn provider_base_uri_for(&self, candidate: &str) -> MResult<Option<String>> {
        let Some((_, base)) = self.select_provider(candidate)? else {
            return Ok(None);
        };
        Ok(Some(
            base.unwrap_or(resource_uri_origin(candidate)?.to_string()),
        ))
    }
    fn select_provider(&self, uri: &str) -> MResult<Option<(usize, Option<String>)>> {
        let scheme = resource_uri_scheme(uri)?.to_string();
        let mut fallback = None;
        let mut best: Option<(usize, String)> = None;
        for (idx, provider) in self.providers.iter().enumerate() {
            if provider.scheme() != scheme {
                continue;
            }
            let bases = provider.base_uris();
            if bases.is_empty() {
                fallback = Some(idx);
                continue;
            }
            for base in bases {
                if resource_base_matches(&base, uri)
                    && best.as_ref().map_or(true, |(_, b)| base.len() > b.len())
                {
                    best = Some((idx, base));
                }
            }
        }
        Ok(best
            .map(|(i, b)| (i, Some(b)))
            .or_else(|| fallback.map(|i| (i, None))))
    }
    pub fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
        let Some((idx, _)) = self.select_provider(&request.base_uri)? else {
            return Err(MechError::new(
                RuntimeResourceProviderNotFound {
                    scheme,
                    uri: request.base_uri,
                },
                None,
            ));
        };
        self.providers[idx].read(request)
    }
    pub fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
        let Some((idx, _)) = self.select_provider(&request.base_uri)? else {
            return Err(MechError::new(
                RuntimeResourceProviderNotFound {
                    scheme,
                    uri: request.base_uri,
                },
                None,
            ));
        };
        self.providers[idx].preflight_write(request)
    }
    pub fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
        let Some((idx, _)) = self.select_provider(&request.base_uri)? else {
            return Err(MechError::new(
                RuntimeResourceProviderNotFound {
                    scheme,
                    uri: request.base_uri,
                },
                None,
            ));
        };
        self.providers[idx].write(request)
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryDocsProvider {
    documents: HashMap<String, HashMap<String, Value>>,
}

impl InMemoryDocsProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self,
        base_uri: impl Into<String>,
        path: impl Into<String>,
        value: Value,
    ) -> MResult<()> {
        let base_uri = base_uri.into();
        let path = path.into();
        let scheme = resource_uri_scheme(&base_uri)?;
        if scheme != "docs" {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: base_uri,
                    reason: "in-memory docs resources require the `docs` scheme".to_string(),
                },
                None,
            ));
        }
        if path.is_empty() {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: base_uri,
                    reason: "resource path cannot be empty".to_string(),
                },
                None,
            ));
        }
        self.documents
            .entry(base_uri)
            .or_default()
            .insert(path, value);
        Ok(())
    }

    pub fn with_value(
        mut self,
        base_uri: impl Into<String>,
        path: impl Into<String>,
        value: Value,
    ) -> MResult<Self> {
        self.insert(base_uri, path, value)?;
        Ok(self)
    }
}

impl RuntimeResourceProvider for InMemoryDocsProvider {
    fn scheme(&self) -> &str {
        "docs"
    }

    fn base_uris(&self) -> Vec<String> {
        self.documents.keys().cloned().collect()
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        let Some(document) = self.documents.get(&request.base_uri) else {
            return Err(MechError::new(
                RuntimeResourcePathNotFound {
                    base_uri: request.base_uri,
                    path: request.path,
                },
                None,
            ));
        };
        let Some(value) = document.get(&request.path) else {
            return Err(MechError::new(
                RuntimeResourcePathNotFound {
                    base_uri: request.base_uri,
                    path: request.path,
                },
                None,
            ));
        };
        Ok(value.clone())
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.intent == RuntimeResourceWriteIntent::Send {
            return Err(MechError::new(
                RuntimeResourceWriteUnsupported {
                    scheme: self.scheme().to_string(),
                    base_uri: request.base_uri,
                    path: request.path,
                },
                None,
            ));
        }

        let scheme = resource_uri_scheme(&request.base_uri)?;
        if scheme != "docs" {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: request.base_uri,
                    reason: "in-memory docs resources require the `docs` scheme".to_string(),
                },
                None,
            ));
        }

        if request.path.is_empty() {
            return Err(MechError::new(
                RuntimeResourceInvalidUri {
                    uri: request.base_uri,
                    reason: "resource path cannot be empty".to_string(),
                },
                None,
            ));
        }

        Ok(())
    }

    fn write(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            intent: request.intent,
        })?;

        self.documents
            .entry(request.base_uri)
            .or_default()
            .insert(request.path, request.value);

        Ok(())
    }
}

pub fn resource_base_matches(base: &str, candidate: &str) -> bool {
    candidate == base
        || candidate
            .strip_prefix(base)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn resource_uri_origin(uri: &str) -> MResult<&str> {
    let scheme = resource_uri_scheme(uri)?;
    let rest = &uri[scheme.len() + 3..];
    let authority_end = rest.find('/').unwrap_or(rest.len());
    if authority_end == 0 {
        return Err(MechError::new(
            RuntimeResourceInvalidUri {
                uri: uri.to_string(),
                reason: "resource URI authority cannot be empty".to_string(),
            },
            None,
        ));
    }
    Ok(&uri[..scheme.len() + 3 + authority_end])
}

fn resource_uri_scheme(uri: &str) -> MResult<&str> {
    let Some((scheme, _rest)) = uri.split_once("://") else {
        return Err(MechError::new(
            RuntimeResourceInvalidUri {
                uri: uri.to_string(),
                reason: "resource URI must contain `://`".to_string(),
            },
            None,
        ));
    };
    if scheme.is_empty() {
        return Err(MechError::new(
            RuntimeResourceInvalidUri {
                uri: uri.to_string(),
                reason: "resource URI scheme cannot be empty".to_string(),
            },
            None,
        ));
    }
    Ok(scheme)
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceInvalidUri {
    pub uri: String,
    pub reason: String,
}

impl MechErrorKind for RuntimeResourceInvalidUri {
    fn name(&self) -> &str {
        "RuntimeResourceInvalidUri"
    }

    fn message(&self) -> String {
        format!("invalid resource URI `{}`: {}", self.uri, self.reason)
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceProviderNotFound {
    pub scheme: String,
    pub uri: String,
}

impl MechErrorKind for RuntimeResourceProviderNotFound {
    fn name(&self) -> &str {
        "RuntimeResourceProviderNotFound"
    }

    fn message(&self) -> String {
        format!(
            "no runtime resource provider registered for scheme `{}` while reading `{}`",
            self.scheme, self.uri,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceWriteUnsupported {
    pub scheme: String,
    pub base_uri: String,
    pub path: String,
}

impl MechErrorKind for RuntimeResourceWriteUnsupported {
    fn name(&self) -> &str {
        "RuntimeResourceWriteUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "runtime resource provider for scheme `{}` does not support writes to `{}` under `{}`",
            self.scheme, self.path, self.base_uri,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceProviderConflict {
    pub scheme: String,
}

impl MechErrorKind for RuntimeResourceProviderConflict {
    fn name(&self) -> &str {
        "RuntimeResourceProviderConflict"
    }

    fn message(&self) -> String {
        format!(
            "runtime resource provider for scheme `{}` is already registered",
            self.scheme
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourcePathNotFound {
    pub base_uri: String,
    pub path: String,
}

impl MechErrorKind for RuntimeResourcePathNotFound {
    fn name(&self) -> &str {
        "RuntimeResourcePathNotFound"
    }

    fn message(&self) -> String {
        format!(
            "resource path `{}` was not found under `{}`",
            self.path, self.base_uri
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceCapabilityDenied {
    pub context_name: String,
    pub operation: String,
    pub path: String,
}

impl MechErrorKind for RuntimeResourceCapabilityDenied {
    fn name(&self) -> &str {
        "RuntimeResourceCapabilityDenied"
    }

    fn message(&self) -> String {
        format!(
            "context `{}` does not allow `{}` on `{}`",
            self.context_name, self.operation, self.path,
        )
    }
}
