use super::catalog::{
    MaterializedHostContext, MaterializedHostInterface, validate_uri_with_authority,
};
use crate::InvalidConfigField;
use mech_core::{MResult, MechError};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostManifestConfig {
    pub provider: String,
    pub contexts: Vec<HostContextManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostContextManifest {
    pub name: String,
    pub base_uri_template: String,
    pub operations: Vec<String>,
}

pub fn validate_host_manifest(manifest: &HostManifestConfig) -> MResult<()> {
    if manifest.provider.trim().is_empty() {
        return invalid("host.provider must be non-empty");
    }
    if manifest.contexts.is_empty() {
        return invalid("host.contexts must contain at least one context");
    }
    let mut names = BTreeSet::new();
    for context in &manifest.contexts {
        if context.name.trim().is_empty() {
            return invalid("host.contexts[].name must be non-empty");
        }
        if !names.insert(context.name.clone()) {
            return invalid(format!("duplicate host context `{}`", context.name));
        }
        if context.base_uri_template.trim().is_empty() {
            return invalid(format!(
                "host context `{}` base-uri must be non-empty",
                context.name
            ));
        }
        if !context.base_uri_template.contains("://") {
            return invalid(format!(
                "host context `{}` base-uri must contain `://`",
                context.name
            ));
        }
        if !context.base_uri_template.contains("{instance}") {
            return invalid(format!(
                "host context `{}` base-uri must contain `{{instance}}`",
                context.name
            ));
        }
        super::validate_host_operations(
            &format!("host context `{}` operations", context.name),
            &context.operations,
        )?;
    }
    Ok(())
}

pub fn materialize_host_manifest(
    instance: &str,
    manifest: &HostManifestConfig,
) -> MResult<MaterializedHostInterface> {
    if instance.trim().is_empty() {
        return invalid("host instance name must be non-empty");
    }
    validate_host_manifest(manifest)?;
    let mut bases = BTreeSet::new();
    let mut contexts = Vec::new();
    for context in &manifest.contexts {
        let base_uri = context.base_uri_template.replace("{instance}", instance);
        validate_uri_with_authority(&base_uri)?;
        if !bases.insert(base_uri.clone()) {
            return invalid(format!("duplicate materialized host base URI `{base_uri}`"));
        }
        contexts.push(MaterializedHostContext {
            name: context.name.clone(),
            base_uri,
            operations: context.operations.clone(),
        });
    }
    Ok(MaterializedHostInterface {
        instance: instance.to_string(),
        provider: manifest.provider.clone(),
        contexts,
    })
}

fn invalid<T>(message: impl Into<String>) -> MResult<T> {
    Err(MechError::new(InvalidConfigField::new(message), None))
}
