//! Shared registration/signature metadata for type-directed source members.
//! Frontend sugar and future completion tooling use this query. It never
//! executes values, calls a specializer/binder, or grants native authority.
use super::FunctionEnvironment;
#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use mech_core::{
    ExternalInteraction, FunctionCatalog, FunctionExposure, FunctionTypeDeclaration,
    InputKindScheme, KindExpr, OperationContractDeclaration, OperationId, ResolvedOutputSchemaRule,
    ResolvedOverload, ResolvedType, SourceInputKind, TypeConstraintOrigin, TypeOverloadCandidate,
    TypeResolutionError,
};
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::{collections::BTreeMap, string::String};

/// The use of receiver metadata, not a separate operation inventory.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReceiverMemberRole {
    /// Source read sugar: a fully resolved, pure, unary, one-value projection.
    #[default]
    UnaryPureProjection,
    /// Tooling: registered signatures whose first source argument can be the
    /// receiver. Remaining arguments, output schemas and contracts still need
    /// ordinary full-call checking; this does not make implicit calls legal.
    Completion,
}

/// Scope metadata only. Exact-name filtering serves underscore reads. A
/// completion caller selects Completion and may omit the exact-name filter.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReceiverMemberQuery<'a> {
    pub role: ReceiverMemberRole,
    /// Canonical exported item, NOT an imported alias. visible_names reports
    /// the currently bound call spellings separately.
    pub exact_name: Option<&'a str>,
    /// Source-module roots shadowing otherwise registered catalog spellings.
    pub excluded_module_roots: &'a [&'a str],
    /// Exact call spellings shadowed by lexical source declarations. Unlike
    /// module roots, excluding `size` does not exclude `size/other`.
    pub excluded_names: &'a [&'a str],
    /// Include hidden, unresolved-receiver or incompatible signatures for
    /// diagnostics. Completion otherwise includes prefix/pending-argument
    /// metadata, not just fully available unary projections.
    pub include_unavailable: bool,
}

#[derive(Clone, Debug)]
pub enum ReceiverMemberCompatibility {
    Available {
        resolution: ResolvedOverload,
        contract: OperationContractDeclaration,
    },
    /// Only the supplied receiver prefix was checked by the ordinary core
    /// solver. No full-call overload, conversions or output was selected.
    PrefixCompatible {
        candidate_ids: Box<[u64]>,
    },
    /// This registered source template needs more arguments before it can
    /// instantiate a checkable scheme. Compatibility is deliberately unknown.
    ArgumentsRequired,
    Ambiguous(TypeResolutionError),
    UnresolvedReceiver,
    Incompatible,
    NotVisible,
}

/// A descriptive source-member candidate, not an execution permission.
#[derive(Clone, Debug)]
pub struct ReceiverMember {
    pub name: String,
    pub canonical_name: String,
    pub module: Option<String>,
    pub operation: OperationId,
    pub visible_names: Box<[String]>,
    pub declaration: FunctionTypeDeclaration,
    /// Original catalog contracts, including multi-output and external
    /// interactions. Tooling must not infer runtime permission from visibility.
    pub contracts: Box<[OperationContractDeclaration]>,
    pub compatibility: ReceiverMemberCompatibility,
}

fn excluded(name: &str, roots: &[&str]) -> bool {
    roots.iter().any(|root| {
        name == *root
            || name
                .strip_prefix(*root)
                .is_some_and(|tail| tail.starts_with('/'))
    })
}

fn value_unary(overload: &mech_core::FunctionTypeOverload) -> bool {
    overload.scheme.outputs().len() == 1
        && overload
            .input_layout
            .iter()
            .all(|kind| *kind == SourceInputKind::Value)
        && match overload.scheme.inputs() {
            InputKindScheme::Fixed(inputs) => inputs.len() == 1,
            InputKindScheme::Variadic {
                prefix,
                min_repetitions,
                ..
            } => prefix.len().saturating_add(*min_repetitions as usize) <= 1,
        }
}

/// A shared post-overload check, used by ordinary calls and member queries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResolvedOutputSchemaRuleSelectionError {
    UnknownRule,
    NoSelection,
    ConflictingRules,
}

pub(crate) fn select_resolved_output_schema_rule<'a>(
    declaration: &'a FunctionTypeDeclaration,
    resolved: &ResolvedOverload,
) -> Result<&'a ResolvedOutputSchemaRule, ResolvedOutputSchemaRuleSelectionError> {
    let rules = resolved
        .candidate_ids
        .iter()
        .map(|candidate| {
            declaration
                .overloads
                .iter()
                .find(|overload| u64::from(overload.id) == *candidate)
                .and_then(|overload| overload.output_schema_rules.first())
                .ok_or(ResolvedOutputSchemaRuleSelectionError::UnknownRule)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let first = rules
        .first()
        .copied()
        .ok_or(ResolvedOutputSchemaRuleSelectionError::NoSelection)?;
    if rules.iter().any(|rule| *rule != first) {
        return Err(ResolvedOutputSchemaRuleSelectionError::ConflictingRules);
    }
    Ok(first)
}

fn completion_compatibility(
    name: &str,
    declaration: &FunctionTypeDeclaration,
    receiver: Option<&ResolvedType>,
) -> ReceiverMemberCompatibility {
    let Some(receiver) = receiver else {
        return ReceiverMemberCompatibility::UnresolvedReceiver;
    };
    if matches!(receiver.kind(), KindExpr::Wildcard | KindExpr::Hole) {
        return ReceiverMemberCompatibility::UnresolvedReceiver;
    }
    let mut declaration = declaration.clone();
    if let Some(template) = declaration.template {
        let Ok(schemes) = mech_core::instantiate_source_scheme_template(
            template,
            core::slice::from_ref(receiver),
        ) else {
            return ReceiverMemberCompatibility::ArgumentsRequired;
        };
        declaration = FunctionTypeDeclaration::from_schemes(schemes);
    }
    let candidates = declaration
        .overloads
        .iter()
        .filter(|overload| overload.input_layout.first() == Some(&SourceInputKind::Value))
        .map(|overload| TypeOverloadCandidate {
            id: u64::from(overload.id),
            scheme: &overload.scheme,
        })
        .collect::<Vec<_>>();
    match mech_core::match_type_overload_prefix(
        TypeConstraintOrigin::new(String::from(name), None),
        &candidates,
        core::slice::from_ref(receiver),
    ) {
        Ok(candidate_ids) => ReceiverMemberCompatibility::PrefixCompatible { candidate_ids },
        Err(error @ TypeResolutionError::Ambiguous { .. }) => {
            ReceiverMemberCompatibility::Ambiguous(error)
        }
        Err(TypeResolutionError::Incompatible { .. }) => ReceiverMemberCompatibility::Incompatible,
    }
}

fn compatibility(
    catalog: &FunctionCatalog,
    name: &str,
    declaration: &FunctionTypeDeclaration,
    receiver: Option<&ResolvedType>,
) -> ReceiverMemberCompatibility {
    let Some(receiver) = receiver else {
        return ReceiverMemberCompatibility::UnresolvedReceiver;
    };
    if matches!(receiver.kind(), KindExpr::Wildcard | KindExpr::Hole) {
        return ReceiverMemberCompatibility::UnresolvedReceiver;
    }
    let mut declaration = declaration.clone();
    if let Some(template) = declaration.template {
        let Ok(schemes) = mech_core::instantiate_source_scheme_template(
            template,
            core::slice::from_ref(receiver),
        ) else {
            return ReceiverMemberCompatibility::Incompatible;
        };
        declaration = FunctionTypeDeclaration::from_schemes(schemes);
    }
    let candidates = declaration
        .overloads
        .iter()
        .filter(|overload| value_unary(overload))
        .map(|overload| TypeOverloadCandidate {
            id: u64::from(overload.id),
            scheme: &overload.scheme,
        })
        .collect::<Vec<_>>();
    let resolved = match mech_core::resolve_type_overloads(
        TypeConstraintOrigin::new(String::from(name), None),
        &candidates,
        core::slice::from_ref(receiver),
        None,
    ) {
        Ok(resolved) => resolved,
        Err(error @ TypeResolutionError::Ambiguous { .. }) => {
            return ReceiverMemberCompatibility::Ambiguous(error);
        }
        Err(TypeResolutionError::Incompatible { .. }) => {
            return ReceiverMemberCompatibility::Incompatible;
        }
    };
    let [output] = resolved.outputs.as_ref() else {
        return ReceiverMemberCompatibility::Incompatible;
    };
    if select_resolved_output_schema_rule(&declaration, &resolved).is_err() {
        return ReceiverMemberCompatibility::Incompatible;
    }
    let Some(contract) = catalog
        .source_operation_contract(name, 1, matches!(output.kind(), KindExpr::Matrix { .. }))
        .filter(|contract| contract.interaction == ExternalInteraction::Pure)
        .cloned()
    else {
        return ReceiverMemberCompatibility::Incompatible;
    };
    ReceiverMemberCompatibility::Available {
        resolution: resolved,
        contract,
    }
}

impl FunctionEnvironment {
    /// Discover source members from THIS selected catalog and THIS import
    /// scope. The role explicitly separates unary read sugar from descriptive
    /// completion signatures, including functions requiring extra arguments.
    /// Every call reflects current registrations
    /// and visible aliases; there is no editor-only or frontend-only inventory.
    ///
    /// A concrete receiver uses the ordinary core kind/dimension/constraint/
    /// conversion overload solver. None returns unresolved signature metadata
    /// when include_unavailable is true. Multiple distinct operations remain
    /// separate candidates, even if their spellings or results coincide.
    pub fn receiver_members(
        &self,
        catalog: &FunctionCatalog,
        receiver: Option<&ResolvedType>,
        query: ReceiverMemberQuery<'_>,
    ) -> Vec<ReceiverMember> {
        let mut members = BTreeMap::new();
        for export in catalog.all_exports() {
            if export.exposure == FunctionExposure::Internal {
                continue;
            }
            let name = export
                .item
                .as_deref()
                .unwrap_or_else(|| export.canonical_name.rsplit('/').next().unwrap_or(""));
            if query.exact_name.is_some_and(|exact| exact != name)
                || members.contains_key(&(String::from(name), export.operation))
            {
                continue;
            }
            let Some(declaration) = catalog
                .source_type_declaration(&export.canonical_name)
                .cloned()
            else {
                continue;
            };
            if declaration.template.is_none()
                && !declaration
                    .overloads
                    .iter()
                    .any(|overload| match query.role {
                        ReceiverMemberRole::UnaryPureProjection => value_unary(overload),
                        ReceiverMemberRole::Completion => {
                            overload.input_layout.first() == Some(&SourceInputKind::Value)
                        }
                    })
            {
                continue;
            }
            if query.role == ReceiverMemberRole::UnaryPureProjection
                && ![false, true].into_iter().any(|matrix| {
                    catalog
                        .source_operation_contract(&export.canonical_name, 1, matrix)
                        .is_some_and(|contract| contract.interaction == ExternalInteraction::Pure)
                })
            {
                continue;
            }
            let visible_names = self
                .visible_operation_names(export.operation)
                .filter(|name| !excluded(name, query.excluded_module_roots))
                .filter(|name| !query.excluded_names.contains(name))
                .map(String::from)
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let compatibility =
                if !self.operation_is_enabled(export.operation) || visible_names.is_empty() {
                    ReceiverMemberCompatibility::NotVisible
                } else {
                    match query.role {
                        ReceiverMemberRole::UnaryPureProjection => {
                            compatibility(catalog, &export.canonical_name, &declaration, receiver)
                        }
                        ReceiverMemberRole::Completion => {
                            completion_compatibility(&export.canonical_name, &declaration, receiver)
                        }
                    }
                };
            if !query.include_unavailable
                && !matches!(
                    compatibility,
                    ReceiverMemberCompatibility::Available { .. }
                        | ReceiverMemberCompatibility::PrefixCompatible { .. }
                        | ReceiverMemberCompatibility::ArgumentsRequired
                        | ReceiverMemberCompatibility::Ambiguous(_)
                )
            {
                continue;
            }
            members.insert(
                (String::from(name), export.operation),
                ReceiverMember {
                    name: String::from(name),
                    canonical_name: export.canonical_name.clone(),
                    module: export.module.clone(),
                    operation: export.operation,
                    visible_names,
                    declaration,
                    contracts: catalog
                        .specializer(export.operation)
                        .map(|entry| entry.operation_contracts().to_vec().into_boxed_slice())
                        .unwrap_or_default(),
                    compatibility,
                },
            );
        }
        members.into_values().collect()
    }
}
