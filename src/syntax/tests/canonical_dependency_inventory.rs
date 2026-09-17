use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use mech_syntax::document::ast::{
    GrammarDefinitionSyntax, GrammarDocumentSyntax, GrammarIdentifierSyntax, GrammarRuleSyntax,
    GrammarSyntax,
};
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSnapshot, TokenFlags,
    parse_canonical_grammar,
};

const EXPECTED_RULES: usize = 539;
const FENCE_OPEN: &str = "```ebnf:canonical";
const FENCE_CLOSE: &str = "```";
const DEPENDENCY_HEADER: &str = "grammar-name\tdirect-children\tdirect-parents";

#[derive(Debug)]
struct DependencyRow {
    children: BTreeSet<String>,
    parents: BTreeSet<String>,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fields(line: &str) -> Vec<&str> {
    line.split('\t').collect()
}

fn parse_name_list(field: &str) -> BTreeSet<String> {
    if field == "none" {
        return BTreeSet::new();
    }
    let names = field
        .split('|')
        .map(str::trim)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    assert!(
        !names.contains(""),
        "dependency list contains an empty name"
    );
    assert_eq!(
        names.len(),
        field.split('|').count(),
        "duplicate dependency"
    );
    names
}

fn read_dependency_rows() -> BTreeMap<String, DependencyRow> {
    let source = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/canonical-dependencies.tsv"),
    )
    .expect("read canonical-dependencies.tsv");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(DEPENDENCY_HEADER));
    let mut previous = String::new();
    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        assert!(!line.is_empty(), "blank dependency row {}", index + 2);
        let row = fields(line);
        assert_eq!(row.len(), 3, "invalid dependency row {}", index + 2);
        assert!(row[0] > previous.as_str(), "dependencies are not ordered");
        previous = row[0].to_owned();
        let value = DependencyRow {
            children: parse_name_list(row[1]),
            parents: parse_name_list(row[2]),
        };
        assert!(rows.insert(row[0].to_owned(), value).is_none());
    }
    assert_eq!(rows.len(), EXPECTED_RULES);
    rows
}

fn canonical_inventory_names() -> BTreeSet<String> {
    let source =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/productions.tsv"))
            .expect("read productions.tsv");
    let mut lines = source.lines();
    let header = fields(lines.next().expect("productions.tsv header"));
    let name = header
        .iter()
        .position(|field| *field == "grammar-name")
        .unwrap();
    let specification = header
        .iter()
        .position(|field| *field == "spec-location")
        .unwrap();
    let names = lines
        .map(fields)
        .filter(|row| row[specification].starts_with("docs/design/specification.mec::"))
        .map(|row| row[name].to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(names.len(), EXPECTED_RULES);
    names
}

fn port_names() -> BTreeSet<String> {
    let source = fs::read_to_string(repository_root().join("docs/design/grammar-audit/ports.tsv"))
        .expect("read ports.tsv");
    let mut lines = source.lines();
    assert_eq!(
        lines.next(),
        Some(
            "grammar-name\tfamily\tsyntax-status\tlowering-status\t\
             node-policy\tphase\tnotes"
        )
    );
    let names = lines
        .map(fields)
        .map(|row| {
            assert_eq!(row.len(), 7);
            row[0].to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(names.len(), EXPECTED_RULES);
    names
}

fn canonical_fence(source: &str) -> String {
    let lines = source.lines().collect::<Vec<_>>();
    let openings = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == FENCE_OPEN).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(openings.len(), 1, "canonical EBNF fence must be unique");
    let start = openings[0];
    let end = (start + 1..lines.len())
        .find(|index| lines[*index] == FENCE_CLOSE)
        .expect("canonical EBNF closing fence");
    let mut fence = lines[start + 1..end].join("\n");
    fence.push('\n');
    fence
}

fn assignment_marker(production: &str) -> usize {
    let mut markers = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut characters = production.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        if character == '"' {
            quoted = true;
        } else if character == ':'
            && characters
                .peek()
                .is_some_and(|(_, following)| *following == '=')
        {
            markers.push(index);
        }
    }
    assert_eq!(markers.len(), 1, "production must have one assignment");
    markers[0]
}

fn normalize_descriptive_primitives(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        if character == '"' {
            quoted = true;
            continue;
        }
        if character != ';' {
            continue;
        }
        let production = &source[start..index];
        let marker = assignment_marker(production);
        let rhs = production[marker + 2..].trim();
        let name = production[..marker].trim();
        let descriptive = rhs.len() >= 2 && rhs.starts_with('?') && rhs.ends_with('?');
        // Canonical grammar filtering makes a terminal containing only a
        // physical space empty. It is dependency-free, so use the same
        // harmless in-memory terminal as descriptive primitives.
        if descriptive || name == "space" {
            output.push_str(&production[..marker + 2]);
            output.push_str(" \"primitive\" ;");
        } else {
            output.push_str(production);
            output.push(';');
        }
        start = index + 1;
    }
    assert!(!quoted, "canonical EBNF has an unclosed terminal");
    assert!(source[start..].trim().is_empty());
    output.push_str(&source[start..]);
    output
}

fn count_nodes(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    usize::from(node.kind() == kind)
        + node
            .children()
            .map(|child| count_nodes(&child, kind))
            .sum::<usize>()
}

fn grammar_identifier_text(identifier: GrammarIdentifierSyntax) -> String {
    identifier
        .name_tokens()
        .into_iter()
        .filter(|token| {
            !token
                .flags()
                .intersects(TokenFlags::TRIVIA | TokenFlags::SYNTHETIC)
        })
        .map(|token| token.text().expect("grammar identifier source"))
        .collect()
}

fn identifier_text(rule: &GrammarRuleSyntax) -> String {
    grammar_identifier_text(rule.name().expect("grammar rule name"))
}

fn collect_definitions(node: &SyntaxNode, definitions: &mut BTreeSet<String>) {
    if let Some(definition) = GrammarDefinitionSyntax::cast(node.clone()) {
        let name = grammar_identifier_text(
            definition
                .identifier()
                .expect("grammar definition identifier"),
        );
        definitions.insert(name);
    }
    for child in node.children() {
        collect_definitions(&child, definitions);
    }
}

fn assert_required(graph: &BTreeMap<String, DependencyRow>, name: &str, required: &[&str]) {
    let children = &graph.get(name).unwrap().children;
    for child in required {
        assert!(
            children.contains(*child),
            "{name} is missing direct child {child}"
        );
    }
}

#[test]
fn canonical_parser_independently_verifies_every_generated_dependency() {
    let specification = fs::read_to_string(repository_root().join("docs/design/specification.mec"))
        .expect("read specification.mec");
    let normalized = normalize_descriptive_primitives(&canonical_fence(&specification));
    let snapshot = parse_canonical_grammar(
        TextSnapshot::new(DocumentId(62), Revision(0), normalized).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        snapshot.diagnostics.is_empty(),
        "canonical grammar diagnostics: {:#?}",
        snapshot.diagnostics.as_slice()
    );
    let syntax = snapshot.syntax();
    assert_eq!(count_nodes(&syntax, SyntaxKind::GrammarDocument), 1);
    assert_eq!(count_nodes(&syntax, SyntaxKind::Grammar), 1);
    assert_eq!(
        count_nodes(&syntax, SyntaxKind::GrammarRule),
        EXPECTED_RULES
    );

    let document = GrammarDocumentSyntax::cast(syntax).expect("GrammarDocument root");
    let grammar: GrammarSyntax = document.grammar().expect("canonical Grammar node");
    let rules = grammar.rules();
    assert_eq!(rules.len(), EXPECTED_RULES);

    let generated = read_dependency_rows();
    let inventory = canonical_inventory_names();
    assert_eq!(port_names(), inventory);
    assert_eq!(
        generated.keys().cloned().collect::<BTreeSet<_>>(),
        inventory
    );

    let mut parsed_names = BTreeSet::new();
    for rule in rules {
        let name = identifier_text(&rule);
        assert!(
            parsed_names.insert(name.clone()),
            "duplicate parsed rule {name}"
        );
        let expression = rule.expression().expect("grammar rule expression");
        let mut definitions = BTreeSet::new();
        collect_definitions(expression.syntax(), &mut definitions);
        definitions.retain(|definition| inventory.contains(definition));
        assert_eq!(
            definitions,
            generated
                .get(&name)
                .unwrap_or_else(|| panic!("generated dependency row for {name:?}"))
                .children,
            "canonical dependency mismatch for {name}"
        );
    }
    assert_eq!(parsed_names, inventory);

    let mut reversed = generated
        .keys()
        .map(|name| (name.clone(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for (parent, row) in &generated {
        for child in &row.children {
            reversed.get_mut(child).unwrap().insert(parent.clone());
        }
    }
    for (name, row) in &generated {
        assert_eq!(row.parents, reversed[name], "reversed parents for {name}");
    }
}

#[test]
fn canonical_dependency_regressions_cover_recursive_core_edges() {
    let graph = read_dependency_rows();
    assert_required(
        &graph,
        "literal",
        &[
            "number",
            "string",
            "atom",
            "boolean",
            "empty",
            "kind-annotation",
        ],
    );
    assert_required(
        &graph,
        "kind",
        &[
            "kind-any",
            "kind-atom",
            "kind-empty",
            "kind-map",
            "kind-matrix",
            "kind-record",
            "kind-scalar",
            "kind-set",
            "kind-table",
            "kind-tuple",
            "kind-kind",
        ],
    );
    assert_required(
        &graph,
        "kind-annotation",
        &["left-angle", "kind-with-option", "right-angle"],
    );
    assert_required(&graph, "kind-with-option", &["kind", "question"]);
    assert_required(&graph, "kind-scalar", &["identifier", "range-expression"]);
    assert_required(
        &graph,
        "var",
        &["prefixed-context-path", "identifier", "kind-annotation"],
    );
    assert_required(
        &graph,
        "matrix-comprehension",
        &["expression", "comprehension-qualifier"],
    );
    assert_required(
        &graph,
        "set-comprehension",
        &["expression", "comprehension-qualifier"],
    );
    assert_required(
        &graph,
        "comprehension-qualifier",
        &["generator", "variable-define", "expression"],
    );
    assert_required(
        &graph,
        "generator",
        &[
            "pattern",
            "generator-arrow",
            "generator-arrow-u",
            "expression",
        ],
    );
    assert_required(
        &graph,
        "variable-define",
        &["tilde", "var", "define-operator", "expression"],
    );
    assert_required(
        &graph,
        "subscript",
        &[
            "swizzle-subscript",
            "dot-subscript",
            "dot-subscript-int",
            "bracket-subscript",
            "brace-subscript",
        ],
    );
}
