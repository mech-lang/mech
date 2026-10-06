//! Canonical document type declarations. Aliases and enums share one ordered
//! environment so annotations, constructors, functions, and patterns resolve
//! the same nominal identities.

use super::*;

#[derive(Clone)]
enum RawTypeDeclaration {
    Alias {
        annotation: KindAnnotationSyntax,
        syntax: SyntaxNode,
    },
    Enum {
        variants: Vec<(String, Option<KindAnnotationSyntax>, SyntaxNode)>,
        syntax: SyntaxNode,
    },
}

fn collect(
    units: &[DocumentUnit],
    declarations: &mut BTreeMap<String, RawTypeDeclaration>,
) -> Result<(), SourceSemanticError> {
    for unit in units {
        let (name, declaration, syntax) = match unit {
            DocumentUnit::Kind(kind) => {
                let name = kind.name().ok_or_else(|| {
                    internal(
                        SourceSemanticAnchor::for_node(kind.syntax()),
                        "kind declaration has no name".to_owned(),
                    )
                })?;
                let annotation = kind.annotation().ok_or_else(|| {
                    internal(
                        SourceSemanticAnchor::for_node(kind.syntax()),
                        "kind declaration has no annotation".to_owned(),
                    )
                })?;
                (
                    node_text(name.syntax())?,
                    RawTypeDeclaration::Alias {
                        annotation,
                        syntax: kind.syntax().clone(),
                    },
                    kind.syntax(),
                )
            }
            DocumentUnit::Enum(enumeration) => {
                let name = enumeration.name().ok_or_else(|| {
                    internal(
                        SourceSemanticAnchor::for_node(enumeration.syntax()),
                        "enum declaration has no name".to_owned(),
                    )
                })?;
                let mut names = BTreeSet::new();
                let variants = enumeration
                    .variants()
                    .into_iter()
                    .map(|variant| {
                        let name = variant.name().ok_or_else(|| {
                            internal(
                                SourceSemanticAnchor::for_node(variant.syntax()),
                                "enum variant has no name".to_owned(),
                            )
                        })?;
                        let name = node_text(name.syntax())?;
                        if !names.insert(name.clone()) {
                            return Err(SourceSemanticError {
                                code: "source-semantics/duplicate-enum-variant",
                                message: format!("enum variant {name} is declared more than once"),
                                anchor: SourceSemanticAnchor::for_node(variant.syntax()),
                            });
                        }
                        Ok((name, variant.payload(), variant.syntax().clone()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                (
                    node_text(name.syntax())?,
                    RawTypeDeclaration::Enum {
                        variants,
                        syntax: enumeration.syntax().clone(),
                    },
                    enumeration.syntax(),
                )
            }
            DocumentUnit::Fence(_, _, nested) => {
                collect(nested, declarations)?;
                continue;
            }
            _ => continue,
        };
        if builtin_kind_named(&name).is_some() || matches!(name.as_str(), "ix" | "index" | "id") {
            return Err(SourceSemanticError {
                code: "source-semantics/builtin-kind-declaration",
                message: format!("kind {name} collides with a builtin kind"),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
        if declarations.insert(name.clone(), declaration).is_some() {
            return Err(SourceSemanticError {
                code: "source-semantics/duplicate-kind-declaration",
                message: format!("kind {name} is declared more than once"),
                anchor: SourceSemanticAnchor::for_node(syntax),
            });
        }
    }
    Ok(())
}

pub(super) fn enum_declarations(
    units: &[DocumentUnit],
) -> Result<Vec<(String, SyntaxNode)>, SourceSemanticError> {
    let mut declarations = BTreeMap::new();
    collect(units, &mut declarations)?;
    Ok(declarations
        .into_iter()
        .filter_map(|(name, declaration)| match declaration {
            RawTypeDeclaration::Enum { syntax, .. } => Some((name, syntax)),
            RawTypeDeclaration::Alias { .. } => None,
        })
        .collect())
}

impl SemanticBuilder {
    pub(super) fn register_document_types(
        &mut self,
        units: &[DocumentUnit],
        nominal_origin: Option<&CanonicalNominalPath>,
    ) -> Result<(), SourceSemanticError> {
        let mut raw = BTreeMap::new();
        collect(units, &mut raw)?;
        let mut pending = raw.keys().cloned().collect::<BTreeSet<_>>();
        let mut resolved = BTreeMap::new();
        while !pending.is_empty() {
            let mut progressed = false;
            for name in pending.iter().cloned().collect::<Vec<_>>() {
                let result = match &raw[&name] {
                    RawTypeDeclaration::Alias { annotation, .. } => {
                        annotation_schema_draft_with_declarations(annotation, &resolved, &pending)
                    }
                    RawTypeDeclaration::Enum { variants, syntax } => {
                        let origin = nominal_origin.ok_or_else(|| SourceSemanticError {
                            code: "source-semantics/nominal-origin-required",
                            message: "enum declarations require the defining package and module namespace".to_owned(),
                            anchor: SourceSemanticAnchor::for_node(syntax),
                        })?;
                        let path = CanonicalNominalPath::new(
                            origin
                                .segments()
                                .iter()
                                .cloned()
                                .chain(std::iter::once(name.clone()))
                                .collect::<Vec<_>>(),
                        )
                        .map_err(|error| {
                            internal(
                                SourceSemanticAnchor::for_node(syntax),
                                format!("invalid enum path: {error:?}"),
                            )
                        })?;
                        let variants = variants
                            .iter()
                            .map(|(variant, payload, syntax)| {
                                let payload = payload
                                    .as_ref()
                                    .map(|payload| {
                                        let payload = annotation_schema_draft_with_declarations(
                                            payload,
                                            &resolved,
                                            &pending,
                                        )?;
                                        if !payload.dimension_parameters.is_empty() {
                                            return Err(SourceSemanticError {
                                                code: "source-semantics/enum-payload-requires-closed-kind",
                                                message: format!(
                                                    "enum variant {variant} requires a closed payload kind"
                                                ),
                                                anchor: SourceSemanticAnchor::for_node(syntax),
                                            });
                                        }
                                        Ok(payload.body)
                                    })
                                    .transpose()?;
                                Ok(mech_core::EnumVariantSchema {
                                    name: variant.clone(),
                                    payload,
                                })
                            })
                            .collect::<Result<Vec<_>, SourceSemanticError>>()?;
                        Ok(SchemaDraft {
                            body: SchemaBody::Enum {
                                key: NominalKey::from_path(NominalKind::Enum, &path),
                                variants: variants.into_boxed_slice(),
                            },
                            dimension_parameters: Box::new([]),
                        })
                    }
                };
                match result {
                    Ok(schema) => {
                        let syntax = match &raw[&name] {
                            RawTypeDeclaration::Alias { syntax, .. }
                            | RawTypeDeclaration::Enum { syntax, .. } => syntax,
                        };
                        schema
                            .clone()
                            .finalize()
                            .map_err(|error| SourceSemanticError {
                                code: "source-semantics/invalid-kind-declaration",
                                message: format!("kind {name} is invalid: {error:?}"),
                                anchor: SourceSemanticAnchor::for_node(syntax),
                            })?;
                        pending.remove(&name);
                        resolved.insert(name, schema);
                        progressed = true;
                    }
                    Err(error) if error.code == "source-semantics/pending-kind-declaration" => {}
                    Err(error) => return Err(error),
                }
            }
            if !progressed {
                let name = pending.iter().next().unwrap();
                let syntax = match &raw[name] {
                    RawTypeDeclaration::Alias { syntax, .. }
                    | RawTypeDeclaration::Enum { syntax, .. } => syntax,
                };
                return Err(SourceSemanticError {
                    code: "source-semantics/cyclic-kind-declaration",
                    message: format!("kind {name} participates in a declaration cycle"),
                    anchor: SourceSemanticAnchor::for_node(syntax),
                });
            }
        }
        self.declared_kinds = resolved;
        for (name, declaration) in &raw {
            let RawTypeDeclaration::Enum { variants, .. } = declaration else {
                continue;
            };
            let schema = self.declared_kinds[name].clone();
            let SchemaBody::Enum {
                variants: schemas, ..
            } = &schema.body
            else {
                unreachable!("resolved enum schema")
            };
            for (ordinal, ((variant, _, _), declared)) in
                variants.iter().zip(schemas.iter()).enumerate()
            {
                self.declared_variants
                    .entry(variant.clone())
                    .or_default()
                    .push(DeclaredEnumVariant {
                        schema: schema.clone(),
                        ordinal: u32::try_from(ordinal).map_err(|_| SourceSemanticError {
                            code: "source-semantics/enum-variant-identity-exhausted",
                            message: format!("enum {name} has too many variants"),
                            anchor: SourceSemanticAnchor::for_node(match declaration {
                                RawTypeDeclaration::Enum { syntax, .. } => syntax,
                                _ => unreachable!(),
                            }),
                        })?,
                        payload: declared.payload.clone(),
                    });
            }
        }
        Ok(())
    }
}
