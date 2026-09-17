use super::{SourceImportAlias, SourceImportDeclaration, SourceImportKind, SourceRequest};

fn is_source_specifier(specifier: &str) -> bool {
    specifier.contains("://")
        || specifier.starts_with("./")
        || specifier.starts_with("../")
        || specifier.ends_with(".mec")
}

pub fn classify_import_specifier(specifier: impl Into<String>) -> SourceImportDeclaration {
    let specifier = specifier.into();
    if let Some(prefix) = specifier.strip_suffix("/*") {
        SourceImportDeclaration {
            specifier: prefix.to_string(),
            alias: None,
            module: None,
            item: None,
            kind: SourceImportKind::Wildcard,
        }
    } else if is_source_specifier(&specifier) {
        SourceImportDeclaration {
            specifier,
            alias: None,
            module: None,
            item: None,
            kind: SourceImportKind::DependencyOnly,
        }
    } else if let Some((module, name)) = specifier.rsplit_once('/') {
        SourceImportDeclaration {
            specifier: module.to_string(),
            alias: None,
            module: None,
            item: None,
            kind: SourceImportKind::Single {
                name: name.to_string(),
            },
        }
    } else {
        SourceImportDeclaration {
            specifier,
            alias: None,
            module: None,
            item: None,
            kind: SourceImportKind::Namespace,
        }
    }
}

pub fn module_namespace_for_import(import: &SourceImportDeclaration) -> Option<String> {
    fn stem_from_specifier(specifier: &str) -> Option<String> {
        let trimmed = specifier.trim_end_matches('/');
        let candidate = trimmed.rsplit('/').next().unwrap_or(trimmed);
        let candidate = candidate.strip_suffix(".mec").unwrap_or(candidate);
        if candidate.is_empty() {
            None
        } else {
            Some(candidate.to_string())
        }
    }

    if import.specifier.trim().is_empty() {
        return None;
    }

    match &import.kind {
        SourceImportKind::Single { .. }
        | SourceImportKind::Wildcard
        | SourceImportKind::Namespace => stem_from_specifier(&import.specifier),
        SourceImportKind::DependencyOnly => {
            let spec = import.specifier.trim();
            if let Some((_, path_part)) = spec.rsplit_once("://") {
                stem_from_specifier(path_part)
            } else {
                stem_from_specifier(spec)
            }
        }
    }
}

pub fn normalize_import_specifier(raw: &str) -> String {
    raw.trim()
        .strip_suffix("/*")
        .unwrap_or(raw.trim())
        .to_string()
}

pub fn source_request_for_import(
    import: &SourceImportDeclaration,
    referrer: Option<&str>,
) -> SourceRequest {
    let mut request = SourceRequest::new(normalize_import_specifier(&import.specifier));
    if let Some(referrer) = referrer {
        request = request.with_referrer(referrer.to_string());
    }
    request
}

pub(super) fn classified_module_import(
    module: &str,
    item: Option<&str>,
    alias: Option<SourceImportAlias>,
) -> SourceImportDeclaration {
    let specifier = match item {
        Some(item) => format!("{module}/{item}"),
        None => module.to_string(),
    };

    let mut declaration = classify_import_specifier(specifier);
    declaration.alias = alias;
    declaration.module = Some(module.to_string());
    declaration.item = item.map(|item| item.to_string());
    declaration
}

pub fn import_requires_source_dependency(import: &SourceImportDeclaration) -> bool {
    if matches!(import.alias, Some(SourceImportAlias::Context(_))) {
        return false;
    }

    matches!(import.kind, SourceImportKind::DependencyOnly)
        || matches!(import.kind, SourceImportKind::Wildcard)
            && is_source_specifier(&import.specifier)
}

pub fn import_may_resolve_source_dependency(import: &SourceImportDeclaration) -> bool {
    !matches!(import.alias, Some(SourceImportAlias::Context(_)))
}

pub fn import_dependencies(imports: &[SourceImportDeclaration]) -> Vec<SourceRequest> {
    imports
        .iter()
        .filter(|import| import_requires_source_dependency(import))
        .map(|import| source_request_for_import(import, None))
        .collect()
}

#[cfg(all(test, feature = "source"))]
mod tests {
    use super::*;

    #[test]
    fn classifies_dependency_only_imports() {
        for specifier in [
            "./dep.mec",
            "../lib/dep.mec",
            "fs://lib/dep.mec",
            "file:///tmp/dep.mec",
            "memory://scratch/dep",
            "https://example.com/dep.mec",
        ] {
            assert_eq!(
                classify_import_specifier(specifier).kind,
                SourceImportKind::DependencyOnly,
                "{specifier}"
            );
        }
    }

    #[test]
    fn namespace_import_does_not_require_source_dependency() {
        let import = classify_import_specifier("math");
        assert_eq!(import.kind, SourceImportKind::Namespace);
        assert!(!import_requires_source_dependency(&import));
    }

    #[test]
    fn single_item_import_does_not_require_source_dependency() {
        let import = classify_import_specifier("math/sin");
        assert!(matches!(import.kind, SourceImportKind::Single { .. }));
        assert!(!import_requires_source_dependency(&import));
    }

    #[test]
    fn compiler_module_wildcard_does_not_require_source_dependency() {
        let import = classify_import_specifier("math/*");
        assert_eq!(import.kind, SourceImportKind::Wildcard);
        assert!(!import_requires_source_dependency(&import));
    }

    #[test]
    fn source_wildcard_requires_source_dependency() {
        for specifier in ["./dep.mec/*", "../lib/dep.mec/*", "file:///tmp/dep.mec/*"] {
            let import = classify_import_specifier(specifier);
            assert_eq!(import.kind, SourceImportKind::Wildcard);
            assert!(import_requires_source_dependency(&import));
        }
    }

    #[test]
    fn relative_file_import_requires_source_dependency() {
        let import = classify_import_specifier("./dep.mec");
        assert_eq!(import.kind, SourceImportKind::DependencyOnly);
        assert!(import_requires_source_dependency(&import));
    }

    #[test]
    fn parent_relative_file_import_requires_source_dependency() {
        let import = classify_import_specifier("../lib/dep.mec");
        assert_eq!(import.kind, SourceImportKind::DependencyOnly);
        assert!(import_requires_source_dependency(&import));
    }

    #[test]
    fn uri_import_requires_source_dependency() {
        let import = classify_import_specifier("file:///tmp/dep.mec");
        assert_eq!(import.kind, SourceImportKind::DependencyOnly);
        assert!(import_requires_source_dependency(&import));
    }

    #[test]
    fn context_import_does_not_require_source_dependency_even_if_dependency_like() {
        let mut import = classify_import_specifier("./host.mec");
        import.alias = Some(SourceImportAlias::Context("host".to_string()));
        assert!(!import_requires_source_dependency(&import));
    }

    #[test]
    fn namespace_import_may_resolve_source_dependency() {
        let import = classify_import_specifier("math");
        assert!(import_may_resolve_source_dependency(&import));
    }

    #[test]
    fn single_item_import_may_resolve_source_dependency() {
        let import = classify_import_specifier("math/sin");
        assert!(import_may_resolve_source_dependency(&import));
    }

    #[test]
    fn wildcard_import_may_resolve_source_dependency() {
        let import = classify_import_specifier("math/*");
        assert!(import_may_resolve_source_dependency(&import));
    }

    #[test]
    fn context_import_may_not_resolve_source_dependency() {
        let mut import = classify_import_specifier("./host.mec");
        import.alias = Some(SourceImportAlias::Context("host".to_string()));
        assert!(!import_may_resolve_source_dependency(&import));
    }

    #[test]
    fn all_imports_create_dependency_edges() {
        let imports = ["math", "math/sin", "math/*", "./dep.mec"].map(classify_import_specifier);
        let dependencies = import_dependencies(&imports);
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].specifier, "./dep.mec");
    }

    #[test]
    fn namespace_for_relative_file_import() {
        let import = classify_import_specifier("./math.mec");
        assert_eq!(
            module_namespace_for_import(&import),
            Some("math".to_string())
        );
    }

    #[test]
    fn namespace_for_parent_relative_file_import() {
        let import = classify_import_specifier("../lib/math.mec");
        assert_eq!(
            module_namespace_for_import(&import),
            Some("math".to_string())
        );
    }

    #[test]
    fn namespace_for_namespace_import() {
        let import = classify_import_specifier("math");
        assert_eq!(
            module_namespace_for_import(&import),
            Some("math".to_string())
        );
    }

    #[test]
    fn namespace_for_single_import() {
        let import = classify_import_specifier("math/tau");
        assert_eq!(
            module_namespace_for_import(&import),
            Some("math".to_string())
        );
    }

    #[test]
    fn namespace_for_wildcard_import() {
        let import = classify_import_specifier("math/*");
        assert_eq!(
            module_namespace_for_import(&import),
            Some("math".to_string())
        );
    }

    #[test]
    fn classifies_mec_wildcard_imports() {
        for (specifier, expected_request) in [
            ("dep.mec/*", "dep.mec"),
            ("./dep.mec/*", "./dep.mec"),
            ("fs://lib/dep.mec/*", "fs://lib/dep.mec"),
        ] {
            let import = classify_import_specifier(specifier);
            assert_eq!(import.kind, SourceImportKind::Wildcard);
            assert_eq!(import.specifier, expected_request);
            assert_eq!(
                source_request_for_import(&import, None).specifier,
                expected_request
            );
        }
    }
}
