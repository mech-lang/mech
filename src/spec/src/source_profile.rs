use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use mech_core::nodes::{
    Expression, MechCode, Paragraph, ParagraphElement, Program, SectionElement, Statement,
};
use serde::{Deserialize, Serialize};

use crate::registry::hash_bytes;
use crate::{Result, SpecError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceProfile {
    Program,
    Configuration,
    Specification,
    Documentation,
}

impl SourceProfile {
    pub fn from_path(path: &Path) -> Result<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("mec") => Ok(Self::Program),
            Some("mcfg") => Ok(Self::Configuration),
            Some("mspec") => Ok(Self::Specification),
            Some("mdoc") => Ok(Self::Documentation),
            extension => Err(SpecError::new(format!(
                "unsupported Mech source profile extension {:?}; expected .mec, .mcfg, .mspec, or .mdoc",
                extension.unwrap_or("<none>")
            ))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentNode {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SymbolDefinition {
    pub identity: String,
    pub name: String,
    pub kind: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticLink {
    pub source_node: String,
    pub relation: String,
    pub symbol_name: String,
    pub symbol_identity: Option<String>,
    pub symbol_kind: Option<String>,
    pub resolved: bool,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentArtifact {
    pub document_id: String,
    pub nodes: Vec<DocumentNode>,
    pub symbols: Vec<SymbolDefinition>,
    pub links: Vec<SemanticLink>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MechArtifact {
    pub profile: SourceProfile,
    pub source_path: String,
    pub source_hash: String,
    pub program_semantic_hash: String,
    pub document: DocumentArtifact,
}

impl MechArtifact {
    pub fn compile(path: &Path) -> Result<Self> {
        let profile = SourceProfile::from_path(path)?;
        let source = fs::read_to_string(path).map_err(|error| {
            SpecError::new(format!("could not read {}: {error}", path.display()))
        })?;
        Self::compile_source(path, profile, &source)
    }

    pub(crate) fn compile_source(
        path: &Path,
        profile: SourceProfile,
        source: &str,
    ) -> Result<Self> {
        let tree = mech_syntax::parser::parse(source).map_err(|error| {
            SpecError::new(format!(
                "unified Mech frontend rejected {}: {error:?}",
                path.display()
            ))
        })?;
        validate_profile(profile, &tree, path)?;
        let source_hash = hash_bytes(source.as_bytes());
        // Document and symbol identities are anchored to the logical source,
        // not its current bytes, so prose-only edits do not silently retarget
        // every semantic link in the document graph.
        let document_hash = hash_bytes(path.to_string_lossy().as_bytes());
        let document_id = format!("doc:{}", &document_hash[..24]);
        let document = build_document(&tree, &document_id);
        let executable = executable_tokens(&tree);
        Ok(Self {
            profile,
            source_path: path.display().to_string(),
            source_hash,
            program_semantic_hash: hash_bytes(executable.as_bytes()),
            document,
        })
    }

    pub fn unresolved_links(&self) -> Vec<&SemanticLink> {
        self.document
            .links
            .iter()
            .filter(|link| !link.resolved)
            .collect()
    }
}

fn validate_profile(profile: SourceProfile, tree: &Program, path: &Path) -> Result<()> {
    let mut violations = Vec::new();
    visit_code(tree, &mut |code| match code {
        MechCode::ActivationScope(_) if profile != SourceProfile::Program => {
            violations.push("activation scope")
        }
        MechCode::FsmImplementation(_) if profile != SourceProfile::Program => {
            violations.push("state-machine implementation")
        }
        MechCode::Statement(Statement::VariableDefine(definition))
            if definition.mutable && profile != SourceProfile::Program =>
        {
            violations.push("mutable variable")
        }
        MechCode::Statement(
            Statement::VariableAssign(_) | Statement::OpAssign(_) | Statement::ContextSend(_),
        ) if profile != SourceProfile::Program => violations.push("external or mutable effect"),
        _ => {}
    });
    if violations.is_empty() {
        Ok(())
    } else {
        violations.sort_unstable();
        violations.dedup();
        Err(SpecError::new(format!(
            "{} profile {:?} prohibits: {}",
            path.display(),
            profile,
            violations.join(", ")
        )))
    }
}

fn build_document(tree: &Program, document_id: &str) -> DocumentArtifact {
    let mut nodes = Vec::new();
    let mut symbols = BTreeMap::new();
    if let Some(title) = &tree.title {
        push_node(
            &mut nodes,
            document_id,
            "title",
            title.text.to_string(),
            span_from_range(&title.text.src_range),
        );
    }
    visit_code(tree, &mut |code| match code {
        MechCode::Statement(Statement::VariableDefine(definition)) => {
            let name = definition.var.name.to_string();
            let span = span_from_range(&definition.var.name.name.src_range);
            symbols.insert(
                name.clone(),
                SymbolDefinition {
                    identity: symbol_identity(document_id, &name),
                    name,
                    kind: definition
                        .var
                        .kind
                        .as_ref()
                        .map(|kind| tokens_text(&kind.tokens()))
                        .filter(|kind| !kind.is_empty())
                        .unwrap_or_else(|| "inferred".to_string()),
                    span,
                },
            );
        }
        MechCode::Statement(Statement::InvariantDefine(definition)) => {
            let name = definition.name.to_string();
            let span = span_from_range(&definition.name.name.src_range);
            symbols.insert(
                name.clone(),
                SymbolDefinition {
                    identity: symbol_identity(document_id, &name),
                    name,
                    kind: "bool".to_string(),
                    span,
                },
            );
        }
        _ => {}
    });

    let mut pending_links = Vec::new();
    for section in &tree.body.sections {
        if let Some(subtitle) = &section.subtitle {
            let span = span_from_paragraph(&subtitle.text);
            push_node(
                &mut nodes,
                document_id,
                "heading",
                subtitle.text.to_string(),
                span,
            );
        }
        for element in &section.elements {
            match element {
                SectionElement::Subtitle(subtitle) => {
                    let span = span_from_paragraph(&subtitle.text);
                    push_node(
                        &mut nodes,
                        document_id,
                        "heading",
                        subtitle.text.to_string(),
                        span,
                    );
                }
                SectionElement::Paragraph(paragraph) => {
                    collect_paragraph(paragraph, document_id, &mut nodes, &mut pending_links)
                }
                _ => {}
            }
        }
    }
    let links = pending_links
        .into_iter()
        .map(|(source_node, name, span)| {
            let definition = symbols.get(&name);
            SemanticLink {
                source_node,
                relation: "references".to_string(),
                symbol_name: name,
                symbol_identity: definition.map(|definition| definition.identity.clone()),
                symbol_kind: definition.map(|definition| definition.kind.clone()),
                resolved: definition.is_some(),
                span,
            }
        })
        .collect();
    DocumentArtifact {
        document_id: document_id.to_string(),
        nodes,
        symbols: symbols.into_values().collect(),
        links,
    }
}

fn collect_paragraph(
    paragraph: &Paragraph,
    document_id: &str,
    nodes: &mut Vec<DocumentNode>,
    links: &mut Vec<(String, String, SourceSpan)>,
) {
    let span = span_from_paragraph(paragraph);
    let node_id = node_identity(document_id, "paragraph", &span, &paragraph.to_string());
    nodes.push(DocumentNode {
        id: node_id.clone(),
        kind: "paragraph".to_string(),
        text: paragraph.to_string(),
        span,
    });
    for element in &paragraph.elements {
        if let ParagraphElement::EvalInlineMechCode(Expression::Var(variable)) = element {
            links.push((
                node_id.clone(),
                variable.name.to_string(),
                span_from_range(&variable.name.name.src_range),
            ));
        }
    }
}

fn push_node(
    nodes: &mut Vec<DocumentNode>,
    document_id: &str,
    kind: &str,
    text: String,
    span: SourceSpan,
) {
    nodes.push(DocumentNode {
        id: node_identity(document_id, kind, &span, &text),
        kind: kind.to_string(),
        text,
        span,
    });
}

fn node_identity(document_id: &str, kind: &str, span: &SourceSpan, text: &str) -> String {
    let identity = format!(
        "{document_id}:{kind}:{}:{}:{}:{}:{text}",
        span.start_line, span.start_column, span.end_line, span.end_column
    );
    format!("node:{}", &hash_bytes(identity.as_bytes())[..24])
}

fn symbol_identity(document_id: &str, name: &str) -> String {
    let identity = hash_bytes(format!("{document_id}:symbol:{name}").as_bytes());
    format!("symbol:{}", &identity[..24])
}

fn span_from_paragraph(paragraph: &Paragraph) -> SourceSpan {
    let tokens = paragraph.tokens();
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => SourceSpan {
            start_line: first.src_range.start.row,
            start_column: first.src_range.start.col,
            end_line: last.src_range.end.row,
            end_column: last.src_range.end.col,
        },
        _ => empty_span(),
    }
}

fn span_from_range(range: &mech_core::nodes::SourceRange) -> SourceSpan {
    SourceSpan {
        start_line: range.start.row,
        start_column: range.start.col,
        end_line: range.end.row,
        end_column: range.end.col,
    }
}

fn empty_span() -> SourceSpan {
    SourceSpan {
        start_line: 0,
        start_column: 0,
        end_line: 0,
        end_column: 0,
    }
}

fn executable_tokens(tree: &Program) -> String {
    let mut output = Vec::new();
    visit_code(tree, &mut |code| output.push(tokens_text(&code.tokens())));
    output.join("\n")
}

fn tokens_text(tokens: &[mech_core::nodes::Token]) -> String {
    tokens.iter().map(|token| token.to_string()).collect()
}

fn visit_code(tree: &Program, visitor: &mut impl FnMut(&MechCode)) {
    for section in &tree.body.sections {
        for element in &section.elements {
            match element {
                SectionElement::MechCode(codes) => {
                    for (code, _) in codes {
                        visitor(code);
                    }
                }
                SectionElement::FencedMechCode(block) => {
                    for (code, _) in &block.code {
                        visitor(code);
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_profiles_share_the_same_parse_and_program_semantics() {
        let source = "Shared source\n=============\n\nvalue := true\n";
        let profiles = [
            ("shared.mec", SourceProfile::Program),
            ("shared.mcfg", SourceProfile::Configuration),
            ("shared.mspec", SourceProfile::Specification),
            ("shared.mdoc", SourceProfile::Documentation),
        ];
        let artifacts = profiles
            .iter()
            .map(|(path, profile)| {
                MechArtifact::compile_source(Path::new(path), *profile, source).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(
            artifacts
                .windows(2)
                .all(|pair| pair[0].program_semantic_hash == pair[1].program_semantic_hash)
        );
    }

    #[test]
    fn specification_links_inline_status_to_the_integrity_symbol() {
        let source = "Status is {contract-status}.\n\ncontract-status<bool> := true\ncontract! := true === true\n";
        let artifact = MechArtifact::compile_source(
            Path::new("demo.mspec"),
            SourceProfile::Specification,
            source,
        )
        .unwrap();
        assert_eq!(artifact.document.links.len(), 1);
        assert!(artifact.document.links[0].resolved);
        assert_eq!(
            artifact.document.links[0].symbol_kind.as_deref(),
            Some("bool")
        );
    }

    #[test]
    fn broken_inline_reference_is_preserved_as_an_unresolved_link() {
        let source = "Status is {missing-status}.\n\ncontract! := true === true\n";
        let artifact = MechArtifact::compile_source(
            Path::new("demo.mspec"),
            SourceProfile::Specification,
            source,
        )
        .unwrap();
        assert_eq!(artifact.unresolved_links().len(), 1);
        assert_eq!(artifact.unresolved_links()[0].symbol_name, "missing-status");
    }

    #[test]
    fn symbol_identity_survives_a_prose_only_edit() {
        let before = MechArtifact::compile_source(
            Path::new("stable.mspec"),
            SourceProfile::Specification,
            "Before.\n\nstatus<bool> := true\n",
        )
        .unwrap();
        let after = MechArtifact::compile_source(
            Path::new("stable.mspec"),
            SourceProfile::Specification,
            "After, with more prose.\n\nstatus<bool> := true\n",
        )
        .unwrap();
        assert_ne!(before.source_hash, after.source_hash);
        assert_eq!(
            before.document.symbols[0].identity,
            after.document.symbols[0].identity
        );
    }

    #[test]
    fn profile_restrictions_run_after_the_shared_parse() {
        let source = "~state := true\n";
        assert!(
            MechArtifact::compile_source(Path::new("state.mec"), SourceProfile::Program, source)
                .is_ok()
        );
        let error = MechArtifact::compile_source(
            Path::new("state.mspec"),
            SourceProfile::Specification,
            source,
        )
        .unwrap_err();
        assert!(error.to_string().contains("prohibits: mutable variable"));
    }
}
