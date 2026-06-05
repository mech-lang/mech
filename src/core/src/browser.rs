#[cfg(feature = "no_std")]
use alloc::collections::BTreeSet;
use core::fmt;
#[cfg(not(feature = "no_std"))]
use std::collections::BTreeSet;

use crate::{MResult, MechError, MechErrorKind};

pub const BROWSER_HOST_IDENTITY: &str = "host://browser";
pub const BROWSER_DOM_PROVIDER_URI: &str = "browser://dom";
pub const BROWSER_CLIPBOARD_PROVIDER_URI: &str = "browser://clipboard";
pub const BROWSER_NETWORK_PROVIDER_URI: &str = "browser://network";
pub const BROWSER_STORAGE_PROVIDER_URI: &str = "browser://storage";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrowserAuthority {
    grants: Vec<BrowserCapabilityGrant>,
}

impl BrowserAuthority {
    pub fn new(grants: impl IntoIterator<Item = BrowserCapabilityGrant>) -> Self {
        let mut out = Self::default();
        for grant in grants {
            out.grant(grant);
        }
        out
    }

    pub fn grants(&self) -> &[BrowserCapabilityGrant] {
        &self.grants
    }

    pub fn grant(&mut self, grant: BrowserCapabilityGrant) {
        if let Some(existing) = self
            .grants
            .iter_mut()
            .find(|existing| existing.resource == grant.resource && existing.budget == grant.budget)
        {
            existing.allow.extend(grant.allow);
        } else {
            self.grants.push(grant);
            self.grants.sort();
        }
    }

    pub fn check(&self, request: &BrowserCapabilityRequest) -> Result<(), BrowserCapabilityError> {
        match request {
            BrowserCapabilityRequest::Dom {
                selector,
                operation,
            } => self.allows_dom(selector, *operation),
            BrowserCapabilityRequest::Clipboard { operation } => self.allows_clipboard(*operation),
            BrowserCapabilityRequest::Network {
                origin,
                method,
                operation,
            } => self.allows_network(origin, method.as_deref(), *operation),
            BrowserCapabilityRequest::Storage {
                backend,
                scope,
                operation,
            } => self.allows_storage(*backend, scope, *operation),
        }
    }

    pub fn allows_dom(
        &self,
        selector: impl AsRef<str>,
        operation: BrowserOperation,
    ) -> Result<(), BrowserCapabilityError> {
        let selector = selector.as_ref();
        BrowserDomScope::validate_selector(selector)?;
        self.check_grant(
            BrowserResourceKind::Dom,
            operation,
            |resource| matches!(resource, BrowserResource::Dom(scope) if scope.selector == selector),
            format!("dom selector `{selector}`"),
        )
    }

    pub fn allows_clipboard(
        &self,
        operation: BrowserOperation,
    ) -> Result<(), BrowserCapabilityError> {
        self.check_grant(
            BrowserResourceKind::Clipboard,
            operation,
            |resource| matches!(resource, BrowserResource::Clipboard),
            "clipboard".to_string(),
        )
    }

    pub fn allows_network(
        &self,
        origin: impl AsRef<str>,
        method: Option<&str>,
        operation: BrowserOperation,
    ) -> Result<(), BrowserCapabilityError> {
        let origin = origin.as_ref();
        BrowserNetworkScope::validate_origin(origin)?;
        let method = method
            .map(BrowserNetworkScope::normalize_method)
            .transpose()?;
        let requested_scope = match method.as_deref() {
            Some(method) => format!("network origin `{origin}` method `{method}`"),
            None => format!("network origin `{origin}`"),
        };
        self.check_grant(
            BrowserResourceKind::Network,
            operation,
            |resource| {
                matches!(
                    resource,
                    BrowserResource::Network(scope)
                        if scope.origin == origin
                            && match (&scope.methods, method.as_deref()) {
                                (Some(methods), Some(method)) => methods.contains(method),
                                (Some(_), None) => false,
                                (None, _) => true,
                            }
                )
            },
            requested_scope,
        )
    }

    pub fn allows_storage(
        &self,
        backend: BrowserStorageBackend,
        scope: impl AsRef<str>,
        operation: BrowserOperation,
    ) -> Result<(), BrowserCapabilityError> {
        let scope = scope.as_ref();
        BrowserStorageScope::validate_scope(scope)?;
        self.check_grant(
            BrowserResourceKind::Storage,
            operation,
            |resource| {
                matches!(resource, BrowserResource::Storage(storage) if storage.backend == backend && storage.scope == scope)
            },
            format!("storage backend `{backend}` scope `{scope}`"),
        )
    }

    fn check_grant(
        &self,
        resource_kind: BrowserResourceKind,
        operation: BrowserOperation,
        mut matches_resource: impl FnMut(&BrowserResource) -> bool,
        scope: String,
    ) -> Result<(), BrowserCapabilityError> {
        let mut saw_resource_kind = false;
        let mut saw_matching_resource = None;

        for grant in &self.grants {
            if grant.resource.kind() == resource_kind {
                saw_resource_kind = true;
            }
            if matches_resource(&grant.resource) {
                saw_matching_resource.get_or_insert_with(|| grant.resource.clone());
                if grant.allow.contains(&operation) {
                    return Ok(());
                }
            }
        }

        if let Some(resource) = saw_matching_resource {
            Err(BrowserCapabilityError::OperationDenied {
                resource,
                operation,
            })
        } else if saw_resource_kind {
            Err(BrowserCapabilityError::NoMatchingGrant {
                resource: resource_kind,
                scope,
            })
        } else {
            Err(BrowserCapabilityError::UnsupportedResource(resource_kind))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BrowserCapabilityGrant {
    pub resource: BrowserResource,
    pub allow: BTreeSet<BrowserOperation>,
    pub budget: Option<BrowserBudget>,
}

impl BrowserCapabilityGrant {
    pub fn new(
        resource: BrowserResource,
        allow: impl IntoIterator<Item = BrowserOperation>,
    ) -> Self {
        Self {
            resource,
            allow: allow.into_iter().collect(),
            budget: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BrowserResource {
    Dom(BrowserDomScope),
    Clipboard,
    Network(BrowserNetworkScope),
    Storage(BrowserStorageScope),
}

impl BrowserResource {
    pub fn kind(&self) -> BrowserResourceKind {
        match self {
            Self::Dom(_) => BrowserResourceKind::Dom,
            Self::Clipboard => BrowserResourceKind::Clipboard,
            Self::Network(_) => BrowserResourceKind::Network,
            Self::Storage(_) => BrowserResourceKind::Storage,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BrowserResourceKind {
    Dom,
    Clipboard,
    Network,
    Storage,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BrowserOperation {
    Read,
    Write,
    List,
    Watch,
    Invoke,
}

impl BrowserOperation {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "list" => Some(Self::List),
            "watch" => Some(Self::Watch),
            "invoke" => Some(Self::Invoke),
            _ => None,
        }
    }
}

impl fmt::Display for BrowserOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::List => "list",
            Self::Watch => "watch",
            Self::Invoke => "invoke",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BrowserDomScope {
    pub selector: String,
}

impl BrowserDomScope {
    pub fn new(selector: impl Into<String>) -> Result<Self, BrowserCapabilityError> {
        let selector = selector.into();
        Self::validate_selector(&selector)?;
        Ok(Self { selector })
    }

    pub fn validate_selector(selector: &str) -> Result<(), BrowserCapabilityError> {
        if selector.trim() != selector || selector.is_empty() {
            return Err(BrowserCapabilityError::InvalidScope {
                resource: BrowserResourceKind::Dom,
                scope: selector.to_string(),
                reason:
                    "DOM selectors must be non-empty and must not include surrounding whitespace"
                        .to_string(),
            });
        }
        if !(selector.starts_with('#') || selector.starts_with('.')) {
            return Err(BrowserCapabilityError::InvalidScope {
                resource: BrowserResourceKind::Dom,
                scope: selector.to_string(),
                reason: "DOM grants must be scoped to a host-provided id or class selector"
                    .to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BrowserNetworkScope {
    pub origin: String,
    pub methods: Option<BTreeSet<String>>,
}

impl BrowserNetworkScope {
    pub fn new(
        origin: impl Into<String>,
        methods: Option<impl IntoIterator<Item = String>>,
    ) -> Result<Self, BrowserCapabilityError> {
        let origin = origin.into();
        Self::validate_origin(&origin)?;
        let methods = methods
            .map(|methods| {
                methods
                    .into_iter()
                    .map(|method| Self::normalize_method(&method))
                    .collect::<Result<BTreeSet<_>, _>>()
            })
            .transpose()?;
        Ok(Self { origin, methods })
    }

    pub fn validate_origin(origin: &str) -> Result<(), BrowserCapabilityError> {
        if origin.trim() != origin || origin.is_empty() {
            return Err(BrowserCapabilityError::InvalidScope {
                resource: BrowserResourceKind::Network,
                scope: origin.to_string(),
                reason:
                    "network origins must be non-empty and must not include surrounding whitespace"
                        .to_string(),
            });
        }
        if origin.ends_with('/')
            || origin.contains('*')
            || !(origin.starts_with("https://") || origin.starts_with("http://"))
        {
            return Err(BrowserCapabilityError::InvalidScope {
                resource: BrowserResourceKind::Network,
                scope: origin.to_string(),
                reason: "network grants must use an http(s) origin without path, trailing slash, or wildcards".to_string(),
            });
        }
        Ok(())
    }

    pub fn normalize_method(method: &str) -> Result<String, BrowserCapabilityError> {
        if method.is_empty()
            || !method
                .bytes()
                .all(|byte| byte.is_ascii_alphabetic() || byte == b'-')
        {
            return Err(BrowserCapabilityError::InvalidScope {
                resource: BrowserResourceKind::Network,
                scope: method.to_string(),
                reason: "network methods must be non-empty HTTP method tokens".to_string(),
            });
        }
        Ok(method.to_ascii_uppercase())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BrowserStorageScope {
    pub backend: BrowserStorageBackend,
    pub scope: String,
}

impl BrowserStorageScope {
    pub fn new(
        backend: BrowserStorageBackend,
        scope: impl Into<String>,
    ) -> Result<Self, BrowserCapabilityError> {
        let scope = scope.into();
        Self::validate_scope(&scope)?;
        Ok(Self { backend, scope })
    }

    pub fn validate_scope(scope: &str) -> Result<(), BrowserCapabilityError> {
        if scope.trim() != scope || scope.is_empty() || scope.contains("..") {
            return Err(BrowserCapabilityError::InvalidScope {
                resource: BrowserResourceKind::Storage,
                scope: scope.to_string(),
                reason: "storage scopes must be non-empty normalized scope strings".to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BrowserStorageBackend {
    LocalStorage,
    SessionStorage,
    IndexedDb,
    Opfs,
}

impl BrowserStorageBackend {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "local-storage" | "localStorage" => Some(Self::LocalStorage),
            "session-storage" | "sessionStorage" => Some(Self::SessionStorage),
            "indexed-db" | "indexeddb" | "indexedDb" => Some(Self::IndexedDb),
            "opfs" => Some(Self::Opfs),
            _ => None,
        }
    }
}

impl fmt::Display for BrowserStorageBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LocalStorage => "local-storage",
            Self::SessionStorage => "session-storage",
            Self::IndexedDb => "indexed-db",
            Self::Opfs => "opfs",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BrowserBudget {
    pub max_invocations: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserCapabilityRequest {
    Dom {
        selector: String,
        operation: BrowserOperation,
    },
    Clipboard {
        operation: BrowserOperation,
    },
    Network {
        origin: String,
        method: Option<String>,
        operation: BrowserOperation,
    },
    Storage {
        backend: BrowserStorageBackend,
        scope: String,
        operation: BrowserOperation,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserCapabilityError {
    Denied {
        reason: String,
    },
    OperationDenied {
        resource: BrowserResource,
        operation: BrowserOperation,
    },
    NoMatchingGrant {
        resource: BrowserResourceKind,
        scope: String,
    },
    InvalidScope {
        resource: BrowserResourceKind,
        scope: String,
        reason: String,
    },
    UnsupportedResource(BrowserResourceKind),
    UnsupportedOperation(BrowserOperation),
    BrowserDeniedOrUnavailable {
        reason: String,
    },
}

impl fmt::Display for BrowserCapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Denied { reason } => write!(f, "browser capability denied: {reason}"),
            Self::OperationDenied {
                resource,
                operation,
            } => {
                write!(
                    f,
                    "browser capability operation `{operation}` denied for {resource:?}"
                )
            }
            Self::NoMatchingGrant { resource, scope } => {
                write!(
                    f,
                    "no matching browser {resource:?} capability grant for {scope}"
                )
            }
            Self::InvalidScope {
                resource,
                scope,
                reason,
            } => {
                write!(f, "invalid browser {resource:?} scope `{scope}`: {reason}")
            }
            Self::UnsupportedResource(resource) => {
                write!(f, "unsupported browser resource {resource:?}")
            }
            Self::UnsupportedOperation(operation) => {
                write!(f, "unsupported browser operation `{operation}`")
            }
            Self::BrowserDeniedOrUnavailable { reason } => {
                write!(f, "browser denied or unavailable: {reason}")
            }
        }
    }
}

#[cfg(not(feature = "no_std"))]
impl std::error::Error for BrowserCapabilityError {}

impl MechErrorKind for BrowserCapabilityError {
    fn name(&self) -> &str {
        match self {
            Self::Denied { .. } => "BrowserCapabilityDenied",
            Self::OperationDenied { .. } => "BrowserCapabilityOperationDenied",
            Self::NoMatchingGrant { .. } => "BrowserCapabilityNoMatchingGrant",
            Self::InvalidScope { .. } => "BrowserCapabilityInvalidScope",
            Self::UnsupportedResource(_) => "BrowserCapabilityUnsupportedResource",
            Self::UnsupportedOperation(_) => "BrowserCapabilityUnsupportedOperation",
            Self::BrowserDeniedOrUnavailable { .. } => "BrowserDeniedOrUnavailable",
        }
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

pub fn browser_capability_error(error: BrowserCapabilityError) -> MechError {
    MechError::new(error, None).with_compiler_loc()
}

pub fn browser_capability_result(result: Result<(), BrowserCapabilityError>) -> MResult<()> {
    result.map_err(browser_capability_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(resource: BrowserResource, allow: &[BrowserOperation]) -> BrowserCapabilityGrant {
        BrowserCapabilityGrant::new(resource, allow.iter().copied())
    }

    #[test]
    fn default_browser_authority_denies_all_resources() {
        let authority = BrowserAuthority::default();
        assert!(matches!(
            authority.allows_clipboard(BrowserOperation::Read),
            Err(BrowserCapabilityError::UnsupportedResource(
                BrowserResourceKind::Clipboard
            ))
        ));
    }

    #[test]
    fn dom_write_allowed_only_for_configured_selector() {
        let authority = BrowserAuthority::new([grant(
            BrowserResource::Dom(BrowserDomScope::new("#mech-output").unwrap()),
            &[BrowserOperation::Write],
        )]);
        assert_eq!(
            authority.allows_dom("#mech-output", BrowserOperation::Write),
            Ok(())
        );
        assert!(matches!(
            authority.allows_dom("#other", BrowserOperation::Write),
            Err(BrowserCapabilityError::NoMatchingGrant { .. })
        ));
    }

    #[test]
    fn clipboard_write_does_not_imply_read() {
        let authority = BrowserAuthority::new([grant(
            BrowserResource::Clipboard,
            &[BrowserOperation::Write],
        )]);
        assert_eq!(authority.allows_clipboard(BrowserOperation::Write), Ok(()));
        assert!(matches!(
            authority.allows_clipboard(BrowserOperation::Read),
            Err(BrowserCapabilityError::OperationDenied { .. })
        ));
    }

    #[test]
    fn clipboard_read_and_write_succeed_when_split_across_budgeted_grants() {
        let mut read = grant(BrowserResource::Clipboard, &[BrowserOperation::Read]);
        read.budget = Some(BrowserBudget {
            max_invocations: Some(1),
        });
        let mut write = grant(BrowserResource::Clipboard, &[BrowserOperation::Write]);
        write.budget = Some(BrowserBudget {
            max_invocations: Some(2),
        });
        let authority = BrowserAuthority::new([read, write]);

        assert_eq!(authority.allows_clipboard(BrowserOperation::Read), Ok(()));
        assert_eq!(authority.allows_clipboard(BrowserOperation::Write), Ok(()));
    }

    #[test]
    fn matching_resource_with_no_allowed_operation_returns_operation_denied() {
        let mut read = grant(BrowserResource::Clipboard, &[BrowserOperation::Read]);
        read.budget = Some(BrowserBudget {
            max_invocations: Some(1),
        });
        let mut list = grant(BrowserResource::Clipboard, &[BrowserOperation::List]);
        list.budget = Some(BrowserBudget {
            max_invocations: Some(2),
        });
        let authority = BrowserAuthority::new([read, list]);

        assert!(matches!(
            authority.allows_clipboard(BrowserOperation::Write),
            Err(BrowserCapabilityError::OperationDenied {
                resource: BrowserResource::Clipboard,
                operation: BrowserOperation::Write,
            })
        ));
    }

    #[test]
    fn resource_kind_with_no_matching_scope_returns_no_matching_grant() {
        let authority = BrowserAuthority::new([grant(
            BrowserResource::Dom(BrowserDomScope::new("#mech-output").unwrap()),
            &[BrowserOperation::Write],
        )]);

        assert!(matches!(
            authority.allows_dom("#other-output", BrowserOperation::Write),
            Err(BrowserCapabilityError::NoMatchingGrant {
                resource: BrowserResourceKind::Dom,
                ..
            })
        ));
    }

    #[test]
    fn network_origin_and_method_restrictions_are_enforced() {
        let authority = BrowserAuthority::new([grant(
            BrowserResource::Network(
                BrowserNetworkScope::new("https://docs.mech-lang.org", Some(["GET".to_string()]))
                    .unwrap(),
            ),
            &[BrowserOperation::Read],
        )]);
        assert_eq!(
            authority.allows_network(
                "https://docs.mech-lang.org",
                Some("get"),
                BrowserOperation::Read
            ),
            Ok(())
        );
        assert!(matches!(
            authority.allows_network("https://example.com", Some("GET"), BrowserOperation::Read),
            Err(BrowserCapabilityError::NoMatchingGrant { .. })
        ));
        assert!(matches!(
            authority.allows_network(
                "https://docs.mech-lang.org",
                Some("POST"),
                BrowserOperation::Read
            ),
            Err(BrowserCapabilityError::NoMatchingGrant { .. })
        ));
    }

    #[test]
    fn storage_grant_distinguishes_backend_and_scope() {
        let authority = BrowserAuthority::new([grant(
            BrowserResource::Storage(
                BrowserStorageScope::new(BrowserStorageBackend::Opfs, "/workspace").unwrap(),
            ),
            &[
                BrowserOperation::Read,
                BrowserOperation::Write,
                BrowserOperation::List,
            ],
        )]);
        assert_eq!(
            authority.allows_storage(
                BrowserStorageBackend::Opfs,
                "/workspace",
                BrowserOperation::List
            ),
            Ok(())
        );
        assert!(matches!(
            authority.allows_storage(
                BrowserStorageBackend::LocalStorage,
                "/workspace",
                BrowserOperation::List
            ),
            Err(BrowserCapabilityError::NoMatchingGrant { .. })
        ));
        assert!(matches!(
            authority.allows_storage(
                BrowserStorageBackend::Opfs,
                "/other",
                BrowserOperation::List
            ),
            Err(BrowserCapabilityError::NoMatchingGrant { .. })
        ));
    }
}
