use std::collections::BTreeSet;

use mech_core::{MResult, MechError, MechErrorKind};

use crate::{
    Capability, CapabilityDecision, CapabilityId, CapabilityRequest, RuntimeCapabilityGrantSpec,
    resource_base_matches,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RuntimeResourceKey {
    pub base_uri: String,
    pub path: String,
}

impl RuntimeResourceKey {
    pub fn new(base_uri: impl Into<String>, path: impl Into<String>) -> MResult<Self> {
        let base_uri = normalize_base_uri(&base_uri.into())?;
        let path = normalize_resource_path(&path.into())?;
        Ok(Self { base_uri, path })
    }

    pub fn capability_resource(&self) -> String {
        if self.path.is_empty() {
            self.base_uri.clone()
        } else {
            format!("{}/{}", self.base_uri, self.path)
        }
    }
}

impl std::fmt::Display for RuntimeResourceKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.capability_resource())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourcePathScope {
    Exact(String),
    Prefix(String),
    Wildcard,
}

impl ResourcePathScope {
    pub(crate) fn from_config_path(path: &str) -> MResult<Self> {
        if path.trim().is_empty() {
            return invalid_resource_capability("configuration path must not be empty");
        }
        if path == "*" {
            return Ok(Self::Wildcard);
        }
        if let Some(prefix) = path.strip_suffix("/*") {
            return Ok(Self::Prefix(normalize_resource_path(prefix)?));
        }
        Ok(Self::Exact(normalize_resource_path(path)?))
    }

    pub(crate) fn config_path(&self) -> String {
        match self {
            Self::Exact(path) => path.clone(),
            Self::Prefix(path) => format!("{path}/*"),
            Self::Wildcard => "*".to_owned(),
        }
    }

    fn matches(&self, path: &str) -> bool {
        match self {
            Self::Exact(exact) => exact == path,
            Self::Prefix(prefix) => {
                path == prefix
                    || path
                        .strip_prefix(prefix)
                        .is_some_and(|remainder| remainder.starts_with('/'))
            }
            Self::Wildcard => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourcePathCapability {
    id: CapabilityId,
    subject: String,
    base_uri: String,
    equivalent_base_uris: BTreeSet<String>,
    operations: BTreeSet<String>,
    scopes: Vec<ResourcePathScope>,
    revocable: bool,
}

impl ResourcePathCapability {
    pub fn new(
        id: CapabilityId,
        subject: impl Into<String>,
        base_uri: impl Into<String>,
        operations: impl IntoIterator<Item = impl Into<String>>,
        scopes: impl IntoIterator<Item = ResourcePathScope>,
    ) -> MResult<Self> {
        let base_uri = normalize_base_uri(&base_uri.into())?;
        let capability = Self {
            id,
            subject: subject.into(),
            base_uri: base_uri.clone(),
            equivalent_base_uris: BTreeSet::from([base_uri]),
            operations: operations.into_iter().map(Into::into).collect(),
            scopes: scopes.into_iter().collect(),
            revocable: true,
        };
        capability.validate()?;
        Ok(capability)
    }

    pub fn from_spec(id: CapabilityId, spec: &RuntimeCapabilityGrantSpec) -> MResult<Self> {
        let scopes = spec
            .paths
            .iter()
            .map(|path| ResourcePathScope::from_config_path(path))
            .collect::<MResult<Vec<_>>>()?;
        Self::new(
            id,
            spec.subject.clone(),
            spec.resource.clone(),
            spec.operations
                .iter()
                .map(|operation| operation.name().to_string()),
            scopes,
        )
    }

    pub fn exact(
        id: CapabilityId,
        subject: impl Into<String>,
        base_uri: impl Into<String>,
        operations: impl IntoIterator<Item = impl Into<String>>,
        path: impl Into<String>,
    ) -> MResult<Self> {
        Self::new(
            id,
            subject,
            base_uri,
            operations,
            [ResourcePathScope::Exact(normalize_resource_path(
                &path.into(),
            )?)],
        )
    }

    pub fn prefix(
        id: CapabilityId,
        subject: impl Into<String>,
        base_uri: impl Into<String>,
        operations: impl IntoIterator<Item = impl Into<String>>,
        path: impl Into<String>,
    ) -> MResult<Self> {
        Self::new(
            id,
            subject,
            base_uri,
            operations,
            [ResourcePathScope::Prefix(normalize_resource_path(
                &path.into(),
            )?)],
        )
    }

    pub fn wildcard(
        id: CapabilityId,
        subject: impl Into<String>,
        base_uri: impl Into<String>,
        operations: impl IntoIterator<Item = impl Into<String>>,
    ) -> MResult<Self> {
        Self::new(
            id,
            subject,
            base_uri,
            operations,
            [ResourcePathScope::Wildcard],
        )
    }

    pub fn base_uri(&self) -> &str {
        &self.base_uri
    }
    pub fn equivalent_base_uris(&self) -> &BTreeSet<String> {
        &self.equivalent_base_uris
    }
    pub fn operations(&self) -> &BTreeSet<String> {
        &self.operations
    }
    pub fn scopes(&self) -> &[ResourcePathScope] {
        &self.scopes
    }

    pub fn with_equivalent_base_uris(
        mut self,
        base_uris: impl IntoIterator<Item = impl Into<String>>,
    ) -> MResult<Self> {
        let primary_scheme = base_uri_scheme(&self.base_uri)?;
        for base_uri in base_uris {
            let base_uri = normalize_base_uri(&base_uri.into())?;
            if base_uri_scheme(&base_uri)? != primary_scheme {
                return invalid_resource_capability(
                    "equivalent base URI scheme must match the primary base URI",
                );
            }
            self.equivalent_base_uris.insert(base_uri);
        }
        self.equivalent_base_uris.insert(self.base_uri.clone());
        self.validate()?;
        Ok(self)
    }

    pub fn revocable(mut self, revocable: bool) -> Self {
        self.revocable = revocable;
        self
    }

    fn check_request(&self, request: &CapabilityRequest) -> MResult<CapabilityDecision> {
        self.validate()?;
        if request.subject != self.subject {
            return Ok(CapabilityDecision::deny(
                "capability belongs to another subject",
            ));
        }
        if !self.operations.contains(&request.operation) {
            return Ok(CapabilityDecision::deny("operation is not allowed"));
        }
        let Some(matching_base) = self
            .equivalent_base_uris
            .iter()
            .filter(|base| resource_base_matches(base, &request.resource))
            .max_by_key(|base| base.len())
        else {
            return Ok(CapabilityDecision::deny("resource base URI is not allowed"));
        };
        let base_prefix = format!("{matching_base}/");
        let path = if request.resource == *matching_base {
            ""
        } else if let Some(path) = request.resource.strip_prefix(&base_prefix) {
            path
        } else {
            return Ok(CapabilityDecision::deny("resource base URI is not allowed"));
        };
        let path = normalize_resource_path(path)?;
        if self.scopes.iter().any(|scope| scope.matches(&path)) {
            Ok(CapabilityDecision::allow())
        } else {
            Ok(CapabilityDecision::deny(
                "resource path is outside the capability scope",
            ))
        }
    }
}

impl Capability for ResourcePathCapability {
    fn id(&self) -> CapabilityId {
        self.id
    }

    fn subject_key(&self) -> &str {
        &self.subject
    }

    fn validate(&self) -> MResult<()> {
        if self.id.is_zero() {
            return invalid_resource_capability("id must not be zero");
        }
        if self.subject.trim().is_empty() {
            return invalid_resource_capability("subject must not be empty");
        }
        if self.base_uri.trim().is_empty() {
            return invalid_resource_capability("base URI must not be empty");
        }
        if !self.equivalent_base_uris.contains(&self.base_uri) {
            return invalid_resource_capability(
                "equivalent base URIs must include the primary base URI",
            );
        }
        let primary_scheme = base_uri_scheme(&self.base_uri)?;
        if self.equivalent_base_uris.iter().any(|base_uri| {
            base_uri.trim().is_empty()
                || base_uri_scheme(base_uri)
                    .map(|scheme| scheme != primary_scheme)
                    .unwrap_or(true)
        }) {
            return invalid_resource_capability(
                "equivalent base URIs must be non-empty and use the primary base URI scheme",
            );
        }
        if self.operations.is_empty()
            || self
                .operations
                .iter()
                .any(|operation| operation.trim().is_empty())
        {
            return invalid_resource_capability("operations must contain non-empty names");
        }
        if self.scopes.is_empty() {
            return invalid_resource_capability("at least one path scope is required");
        }
        Ok(())
    }

    fn check(&self, request: &CapabilityRequest) -> MResult<CapabilityDecision> {
        self.check_request(request)
    }

    fn preview_check(&self, request: &CapabilityRequest) -> MResult<CapabilityDecision> {
        self.check_request(request)
    }

    fn is_revocable(&self) -> bool {
        self.revocable
    }
}

fn normalize_base_uri(base_uri: &str) -> MResult<String> {
    crate::canonicalize_resource_base_uri(base_uri.trim()).map_err(|error| {
        MechError::new(
            RuntimeCapabilityGrantInvalid {
                reason: format!("invalid base URI: {}", error.full_chain_message(),),
            },
            None,
        )
    })
}

fn base_uri_scheme(base_uri: &str) -> MResult<&str> {
    base_uri
        .split_once("://")
        .map(|(scheme, _)| scheme)
        .filter(|scheme| !scheme.is_empty())
        .ok_or_else(|| {
            MechError::new(
                RuntimeCapabilityGrantInvalid {
                    reason: "invalid base URI: resource URI must contain a non-empty scheme"
                        .to_string(),
                },
                None,
            )
        })
}

fn normalize_resource_path(path: &str) -> MResult<String> {
    let mut segments = Vec::new();
    for segment in path.trim().split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                return invalid_resource_capability("resource path must not contain `..`");
            }
            segment => segments.push(segment),
        }
    }
    Ok(segments.join("/"))
}

fn invalid_resource_capability<T>(reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        RuntimeCapabilityGrantInvalid {
            reason: reason.into(),
        },
        None,
    ))
}

#[derive(Debug, Clone)]
pub struct RuntimeCapabilityGrantInvalid {
    pub reason: String,
}

impl MechErrorKind for RuntimeCapabilityGrantInvalid {
    fn name(&self) -> &str {
        "RuntimeCapabilityGrantInvalid"
    }

    fn message(&self) -> String {
        format!("invalid runtime capability grant: {}", self.reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_scope_checks_path_boundaries() {
        let capability = ResourcePathCapability::prefix(
            CapabilityId(1),
            "task:1",
            "docs://manual",
            ["read"],
            "chapter",
        )
        .unwrap();

        assert!(
            capability
                .preview_check(&CapabilityRequest::from_keys(
                    "task:1",
                    "read",
                    "docs://manual/chapter/one",
                ))
                .unwrap()
                .allowed
        );
        assert!(
            !capability
                .preview_check(&CapabilityRequest::from_keys(
                    "task:1",
                    "read",
                    "docs://manual/chapter-two",
                ))
                .unwrap()
                .allowed
        );
    }

    #[test]
    fn exact_and_wildcard_scopes_use_canonical_full_resources() {
        let exact = ResourcePathCapability::exact(
            CapabilityId(1),
            "task:1",
            "docs://manual/intro",
            ["read"],
            "title",
        )
        .unwrap();
        let wildcard =
            ResourcePathCapability::wildcard(CapabilityId(2), "task:1", "docs://manual", ["read"])
                .unwrap();

        let title = CapabilityRequest::from_keys("task:1", "read", "docs://manual/intro/title");
        let sibling = CapabilityRequest::from_keys("task:1", "read", "docs://manual/intro/other");

        assert!(exact.preview_check(&title).unwrap().allowed);
        assert!(!exact.preview_check(&sibling).unwrap().allowed);
        assert!(wildcard.preview_check(&title).unwrap().allowed);
    }

    #[test]
    fn resource_key_rejects_parent_segments() {
        let error = RuntimeResourceKey::new("docs://manual/", "intro/../secret").unwrap_err();
        assert_eq!(error.kind_name(), "RuntimeCapabilityGrantInvalid");
    }

    #[test]
    fn resource_path_capability_authorizes_declared_base_aliases() {
        let capability = ResourcePathCapability::wildcard(
            CapabilityId(1),
            "runtime:1",
            "cli://cli/stdout",
            ["write"],
        )
        .unwrap()
        .with_equivalent_base_uris(["cli://cli/stdout", "cli://stdout/"])
        .unwrap();

        for resource in ["cli://cli/stdout/line", "cli://stdout/line"] {
            assert!(
                capability
                    .preview_check(&CapabilityRequest::from_keys(
                        "runtime:1",
                        "write",
                        resource,
                    ))
                    .unwrap()
                    .allowed
            );
        }
        assert_eq!(
            capability.equivalent_base_uris(),
            &BTreeSet::from(["cli://cli/stdout".to_string(), "cli://stdout".to_string(),]),
        );
    }

    #[test]
    fn resource_path_capability_aliases_preserve_exact_path_scope() {
        let capability = ResourcePathCapability::exact(
            CapabilityId(1),
            "runtime:1",
            "cli://cli/stdout",
            ["write"],
            "line",
        )
        .unwrap()
        .with_equivalent_base_uris(["cli://stdout"])
        .unwrap();

        assert!(
            capability
                .preview_check(&CapabilityRequest::from_keys(
                    "runtime:1",
                    "write",
                    "cli://stdout/line",
                ))
                .unwrap()
                .allowed
        );
        assert!(
            !capability
                .preview_check(&CapabilityRequest::from_keys(
                    "runtime:1",
                    "write",
                    "cli://stdout/text",
                ))
                .unwrap()
                .allowed
        );
        assert!(
            !capability
                .preview_check(&CapabilityRequest::from_keys(
                    "runtime:1",
                    "write",
                    "cli://cli/stdout-other/line",
                ))
                .unwrap()
                .allowed
        );
    }

    #[test]
    fn resource_path_capability_does_not_infer_same_scheme_aliases() {
        let capability = ResourcePathCapability::wildcard(
            CapabilityId(1),
            "runtime:1",
            "cli://cli/stdout",
            ["write"],
        )
        .unwrap()
        .with_equivalent_base_uris(["cli://stdout"])
        .unwrap();

        for resource in [
            "cli://cli/stderr/line",
            "cli://stderr/line",
            "cli://terminal/stdout/line",
        ] {
            assert!(
                !capability
                    .preview_check(&CapabilityRequest::from_keys(
                        "runtime:1",
                        "write",
                        resource,
                    ))
                    .unwrap()
                    .allowed
            );
        }
    }
}
