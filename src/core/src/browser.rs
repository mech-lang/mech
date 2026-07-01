#[cfg(feature = "no_std")]
use alloc::{
    collections::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;
#[cfg(not(feature = "no_std"))]
use std::collections::BTreeSet;

use crate::{MResult, MechError, MechErrorKind};

pub const BROWSER_HOST_IDENTITY: &str = "host://browser";
pub const BROWSER_DOM_PROVIDER_URI: &str = "browser://dom";
pub const BROWSER_CLIPBOARD_PROVIDER_URI: &str = "browser://clipboard";
pub const BROWSER_NETWORK_PROVIDER_URI: &str = "browser://network";
pub const BROWSER_STORAGE_PROVIDER_URI: &str = "browser://storage";

/// Typed browser host grant model used by the WASM facade.
///
/// This is intentionally a compact browser-specific authority for initial host
/// configuration and checks. It does not yet implement runtime capability IDs,
/// delegation, or attenuation through the shared capability kernel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrowserAuthority {
    grants: Vec<BrowserCapabilityGrant>,
    dom_manifest: Vec<BrowserDomManifestEntry>,
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

    pub fn dom_manifest(&self) -> &[BrowserDomManifestEntry] {
        &self.dom_manifest
    }

    pub fn bind_dom_path(&mut self, entry: BrowserDomManifestEntry) {
        if let Some(existing) = self
            .dom_manifest
            .iter_mut()
            .find(|existing| existing.path == entry.path)
        {
            *existing = entry;
        } else {
            self.dom_manifest.push(entry);
            self.dom_manifest
                .sort_by(|left, right| left.path.cmp(&right.path));
        }
    }

    pub fn dom_entry_for_path(&self, path: &BrowserDomPath) -> Option<&BrowserDomManifestEntry> {
        if let Some(exact) = self
            .dom_manifest
            .iter()
            .find(|entry| !entry.path.is_wildcard() && entry.path == *path)
        {
            return Some(exact);
        }

        self
            .dom_manifest
            .iter()
            .filter(|entry| entry.path.is_wildcard() && entry.path.matches(path))
            .max_by_key(|entry| entry.path.as_str().trim_end_matches("/*").len())
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
                matches!(resource, BrowserResource::Storage(storage) if storage.backend == backend && storage.matches_scope(scope))
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
        if !browser_resource_allows_operation(resource_kind, operation) {
            return Err(BrowserCapabilityError::UnsupportedOperation(operation));
        }

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

impl BrowserResourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dom => "dom",
            Self::Clipboard => "clipboard",
            Self::Network => "network",
            Self::Storage => "storage",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "dom" => Some(Self::Dom),
            "clipboard" => Some(Self::Clipboard),
            "network" => Some(Self::Network),
            "storage" => Some(Self::Storage),
            _ => None,
        }
    }
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
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::List => "list",
            Self::Watch => "watch",
            Self::Invoke => "invoke",
        }
    }

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
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserDomManifestEntry {
    pub path: BrowserDomPath,
    pub selector: BrowserDomScope,
    pub property: BrowserDomProperty,
    pub operations: BTreeSet<BrowserOperation>,
}

impl BrowserDomManifestEntry {
    pub fn new(
        path: BrowserDomPath,
        selector: BrowserDomScope,
        property: BrowserDomProperty,
        operations: impl IntoIterator<Item = BrowserOperation>,
    ) -> Self {
        Self {
            path,
            selector,
            property,
            operations: operations.into_iter().collect(),
        }
    }

    pub fn allows_operation(&self, operation: BrowserOperation) -> bool {
        self.operations.contains(&operation)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BrowserDomPath {
    path: String,
    wildcard: bool,
}

impl BrowserDomPath {
    pub fn new(path: impl Into<String>) -> Result<Self, BrowserCapabilityError> {
        let path = path.into();
        validate_browser_dom_path(&path)?;
        let wildcard = path.ends_with("/*");
        Ok(Self { path, wildcard })
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }

    pub fn is_wildcard(&self) -> bool {
        self.wildcard
    }

    pub fn matches(&self, requested: &BrowserDomPath) -> bool {
        if self.path == requested.path {
            return true;
        }

        if !self.wildcard {
            return false;
        }

        let prefix = self
            .path
            .strip_suffix("/*")
            .expect("wildcard DOM path must end in /*");

        requested
            .path
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('/'))
    }

    pub fn join(&self, child: &str) -> Result<Self, BrowserCapabilityError> {
        let base = self.path.trim_end_matches('/');
        let child = child.trim_start_matches('/');
        if base.is_empty() {
            Self::new(child.to_string())
        } else if child.is_empty() {
            Self::new(base.to_string())
        } else {
            Self::new(format!("{base}/{child}"))
        }
    }

    pub fn without_property_suffix(&self) -> &str {
        let Some((base, leaf)) = self.path.rsplit_once('/') else {
            return &self.path;
        };
        if leaf.starts_with('_') {
            base
        } else {
            &self.path
        }
    }

    pub fn dom_property(&self) -> BrowserDomProperty {
        let Some((_, leaf)) = self.path.rsplit_once('/') else {
            return BrowserDomProperty::Text;
        };

        BrowserDomProperty::from_path_segment(leaf)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BrowserDomProperty {
    Text,
    Value,
    InnerHtml,
    Attribute(String),
}

impl BrowserDomProperty {
    pub fn config_name(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Value => "value",
            Self::InnerHtml => "inner-html",
            Self::Attribute(_) => "attribute",
        }
    }

    pub fn config_attribute(&self) -> Option<&str> {
        match self {
            Self::Attribute(attribute) => Some(attribute.as_str()),
            _ => None,
        }
    }

    pub fn parse_config_name(
        property: &str,
        attribute: Option<&str>,
    ) -> Result<Self, BrowserCapabilityError> {
        let path = BrowserDomPath::new("_text")?;
        Self::parse_manifest(Some(property), attribute, &path)
    }

    pub fn from_path_segment(segment: &str) -> Self {
        match segment {
            "_text" => Self::Text,
            "_value" => Self::Value,
            "_html" => Self::InnerHtml,
            segment if segment.starts_with('_') => {
                Self::Attribute(segment.trim_start_matches('_').to_string())
            }
            _ => Self::Text,
        }
    }

    pub fn parse_manifest(
        property: Option<&str>,
        attribute: Option<&str>,
        path: &BrowserDomPath,
    ) -> Result<Self, BrowserCapabilityError> {
        let invalid = |reason: String| BrowserCapabilityError::InvalidScope {
            resource: BrowserResourceKind::Dom,
            scope: path.as_str().to_string(),
            reason,
        };

        match property {
            Some("text") => {
                if attribute.is_some() {
                    return Err(invalid("DOM property `text` cannot include `attribute`".to_string()));
                }
                Ok(Self::Text)
            }
            Some("value") => {
                if attribute.is_some() {
                    return Err(invalid("DOM property `value` cannot include `attribute`".to_string()));
                }
                Ok(Self::Value)
            }
            Some("inner-html") | Some("innerHtml") | Some("html") => {
                if attribute.is_some() {
                    return Err(invalid("DOM property `inner-html` cannot include `attribute`".to_string()));
                }
                Ok(Self::InnerHtml)
            }
            Some("attribute") => {
                let Some(attribute) = attribute else {
                    return Err(invalid("DOM property `attribute` requires an `attribute` name".to_string()));
                };
                validate_dom_attribute_name(attribute)?;
                Ok(Self::Attribute(attribute.to_string()))
            }
            Some(other) => Err(invalid(format!(
                "DOM property `{other}` must be `text`, `value`, `inner-html`, or `attribute`"
            ))),
            None => {
                if attribute.is_some() {
                    return Err(invalid(
                        "`attribute` is only valid when `property` is `attribute`; underscore path segments infer attributes directly".to_string(),
                    ));
                }
                Ok(path.dom_property())
            }
        }
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
        let token = &selector[1..];
        if token.is_empty()
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(BrowserCapabilityError::InvalidScope {
                resource: BrowserResourceKind::Dom,
                scope: selector.to_string(),
                reason: "DOM selector tokens may contain only ASCII letters, digits, `_`, or `-` after `#` or `.`".to_string(),
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
        let invalid = |reason: &str| BrowserCapabilityError::InvalidScope {
            resource: BrowserResourceKind::Network,
            scope: origin.to_string(),
            reason: reason.to_string(),
        };

        if origin.trim() != origin || origin.is_empty() {
            return Err(invalid(
                "network origins must be non-empty and must not include surrounding whitespace",
            ));
        }
        if origin.chars().any(char::is_whitespace) {
            return Err(invalid("network origins must not contain whitespace"));
        }

        let rest = origin
            .strip_prefix("https://")
            .or_else(|| origin.strip_prefix("http://"))
            .ok_or_else(|| invalid("network grants must use an http(s) origin"))?;

        if rest.is_empty() {
            return Err(invalid("network origins must include a host"));
        }
        if rest.contains(['/', '?', '#', '@', '*']) {
            return Err(invalid(
                "network origins must not include path, query, fragment, userinfo, or wildcards",
            ));
        }

        let (host, port) = match rest.rsplit_once(':') {
            Some((host, port)) => {
                if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(invalid("network origin ports must be numeric"));
                }
                let port = port
                    .parse::<u16>()
                    .map_err(|_| invalid("network origin ports must be in 1..65535"))?;
                if port == 0 {
                    return Err(invalid("network origin port 0 is not allowed"));
                }
                (host, Some(port))
            }
            None => (rest, None),
        };
        let _ = port;

        if !is_valid_browser_origin_host(host) {
            return Err(invalid("network origins must include a valid simple host"));
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
    pub recursive: bool,
}

impl BrowserStorageScope {
    pub fn new(
        backend: BrowserStorageBackend,
        scope: impl Into<String>,
    ) -> Result<Self, BrowserCapabilityError> {
        let scope = scope.into();
        Self::validate_scope(&scope)?;
        Ok(Self {
            backend,
            scope,
            recursive: false,
        })
    }

    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    pub fn matches_scope(&self, requested: &str) -> bool {
        if self.scope == requested {
            return true;
        }
        if !self.recursive {
            return false;
        }
        if self.scope == "/" {
            return requested.starts_with('/');
        }
        requested
            .strip_prefix(&self.scope)
            .is_some_and(|suffix| suffix.starts_with('/'))
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

fn validate_browser_dom_path(path: &str) -> Result<(), BrowserCapabilityError> {
    let invalid = |reason: &str| BrowserCapabilityError::InvalidScope {
        resource: BrowserResourceKind::Dom,
        scope: path.to_string(),
        reason: reason.to_string(),
    };

    if path.trim() != path || path.is_empty() {
        return Err(invalid(
            "DOM resource paths must be non-empty and must not include surrounding whitespace",
        ));
    }

    let segments: Vec<&str> = path.split('/').collect();
    for (index, segment) in segments.iter().enumerate() {
        if segment.is_empty() || *segment == "." || *segment == ".." {
            return Err(invalid(
                "DOM resource path segments must be non-empty normalized tokens",
            ));
        }

        if *segment == "*" {
            if index + 1 != segments.len() {
                return Err(invalid(
                    "DOM resource wildcard `*` is only allowed as the final segment",
                ));
            }
            continue;
        }

        if segment.contains('*') {
            return Err(invalid(
                "DOM resource wildcard `*` must be its own final segment",
            ));
        }

        if !segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(invalid(
                "DOM resource path segments may contain only ASCII letters, digits, `_`, or `-`",
            ));
        }
    }

    Ok(())
}

fn validate_dom_attribute_name(attribute: &str) -> Result<(), BrowserCapabilityError> {
    if attribute.trim() != attribute
        || attribute.is_empty()
        || !attribute
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(BrowserCapabilityError::InvalidScope {
            resource: BrowserResourceKind::Dom,
            scope: attribute.to_string(),
            reason: "DOM attribute names must be non-empty simple tokens".to_string(),
        });
    }

    Ok(())
}

fn browser_resource_allows_operation(
    resource: BrowserResourceKind,
    operation: BrowserOperation,
) -> bool {
    match resource {
        BrowserResourceKind::Dom | BrowserResourceKind::Clipboard => {
            matches!(operation, BrowserOperation::Read | BrowserOperation::Write)
        }
        BrowserResourceKind::Network => matches!(operation, BrowserOperation::Read),
        BrowserResourceKind::Storage => matches!(
            operation,
            BrowserOperation::Read | BrowserOperation::Write | BrowserOperation::List
        ),
    }
}

fn is_valid_browser_origin_host(host: &str) -> bool {
    if host.is_empty()
        || host.starts_with('.')
        || host.ends_with('.')
        || host.contains("..")
        || host.contains(['[', ']'])
    {
        return false;
    }

    host.split('.').all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
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
    fn invalid_dom_selector_forms_are_rejected() {
        for selector in [
            "#mech-output, body",
            "body",
            ".mech-output *",
            "#root + body",
            "#root[data-x]",
            "#root:hover",
            "#",
        ] {
            assert!(matches!(
                BrowserDomScope::new(selector),
                Err(BrowserCapabilityError::InvalidScope {
                    resource: BrowserResourceKind::Dom,
                    ..
                })
            ));
        }
    }


    #[test]
    fn dom_manifest_exact_path_beats_wildcard() {
        let mut authority = BrowserAuthority::default();
        let wildcard_scope = BrowserDomScope::new("#wild").unwrap();
        let exact_scope = BrowserDomScope::new("#exact").unwrap();
        authority.bind_dom_path(BrowserDomManifestEntry::new(
            BrowserDomPath::new("body/*").unwrap(),
            wildcard_scope,
            BrowserDomProperty::Text,
            [BrowserOperation::Read, BrowserOperation::Write],
        ));
        authority.bind_dom_path(BrowserDomManifestEntry::new(
            BrowserDomPath::new("body/title").unwrap(),
            exact_scope,
            BrowserDomProperty::Text,
            [BrowserOperation::Read, BrowserOperation::Write],
        ));
        let entry = authority
            .dom_entry_for_path(&BrowserDomPath::new("body/title").unwrap())
            .unwrap();
        assert_eq!(entry.selector.selector, "#exact");
    }

    #[test]
    fn dom_manifest_longest_wildcard_wins() {
        let mut authority = BrowserAuthority::default();
        let short_scope = BrowserDomScope::new("#short").unwrap();
        let long_scope = BrowserDomScope::new("#long").unwrap();
        authority.bind_dom_path(BrowserDomManifestEntry::new(
            BrowserDomPath::new("body/*").unwrap(),
            short_scope,
            BrowserDomProperty::Text,
            [BrowserOperation::Read, BrowserOperation::Write],
        ));
        authority.bind_dom_path(BrowserDomManifestEntry::new(
            BrowserDomPath::new("body/content/*").unwrap(),
            long_scope,
            BrowserDomProperty::Text,
            [BrowserOperation::Read, BrowserOperation::Write],
        ));
        let entry = authority
            .dom_entry_for_path(&BrowserDomPath::new("body/content/title").unwrap())
            .unwrap();
        assert_eq!(entry.selector.selector, "#long");
    }

    #[test]
    fn dom_manifest_sibling_wildcard_does_not_match() {
        let mut authority = BrowserAuthority::default();
        authority.bind_dom_path(BrowserDomManifestEntry::new(
            BrowserDomPath::new("body/content/*").unwrap(),
            BrowserDomScope::new("#content").unwrap(),
            BrowserDomProperty::Text,
            [BrowserOperation::Read, BrowserOperation::Write],
        ));
        assert!(authority
            .dom_entry_for_path(&BrowserDomPath::new("body/sidebar/title").unwrap())
            .is_none());
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
    fn invalid_network_origin_forms_are_rejected() {
        for origin in [
            "https://",
            "https://example.com/path",
            "https://example.com?x=1",
            "https://example.com#frag",
            "https://user@example.com",
            "https://*.example.com",
        ] {
            assert!(matches!(
                BrowserNetworkScope::new(origin, None::<Vec<String>>),
                Err(BrowserCapabilityError::InvalidScope {
                    resource: BrowserResourceKind::Network,
                    ..
                })
            ));
        }
    }

    #[test]
    fn recursive_storage_matching_uses_path_boundaries() {
        let scope = BrowserStorageScope::new(BrowserStorageBackend::Opfs, "/workspace")
            .unwrap()
            .with_recursive(true);

        assert!(scope.matches_scope("/workspace"));
        assert!(scope.matches_scope("/workspace/main.mec"));
        assert!(!scope.matches_scope("/workspace2/main.mec"));
    }

    #[test]
    fn direct_api_invalid_resource_operation_pairs_are_rejected() {
        let network = BrowserAuthority::new([grant(
            BrowserResource::Network(
                BrowserNetworkScope::new("https://docs.mech-lang.org", Some(["GET".to_string()]))
                    .unwrap(),
            ),
            &[BrowserOperation::Write],
        )]);
        assert!(matches!(
            network.allows_network(
                "https://docs.mech-lang.org",
                Some("GET"),
                BrowserOperation::Write
            ),
            Err(BrowserCapabilityError::UnsupportedOperation(
                BrowserOperation::Write
            ))
        ));

        let storage = BrowserAuthority::new([grant(
            BrowserResource::Storage(
                BrowserStorageScope::new(BrowserStorageBackend::Opfs, "/workspace").unwrap(),
            ),
            &[BrowserOperation::Watch],
        )]);
        assert!(matches!(
            storage.allows_storage(
                BrowserStorageBackend::Opfs,
                "/workspace",
                BrowserOperation::Watch
            ),
            Err(BrowserCapabilityError::UnsupportedOperation(
                BrowserOperation::Watch
            ))
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
