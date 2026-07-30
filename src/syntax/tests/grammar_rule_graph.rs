use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

const INVENTORY_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/design/grammar-audit/productions.tsv"
);
const SPECIFICATION_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/design/specification.mec"
);
const EXPECTED_CANONICAL_RULES: usize = 539;
const EXPECTED_NESTED_EBNF_EXAMPLES: usize = 2;

// Public, independently useful lower-level parsers which are not called by
// any of the seven complete/prefix roots in the canonical rule graph.
const DECLARED_SEPARATE_ROOTS: &[&str] =
    &["gen-operator", "match-expression", "synth-operator", "tag"];

// Implemented baseline rules with no accepted-language caller at PR 680's
// audited head. Keeping this list explicit makes accidental reachability drift
// visible without rejecting expected recursion elsewhere in the graph.
const DECLARED_LEGACY_OR_DEAD: &[&str] = &[
    "newline-indent",
    "strike-sigil",
    "table-column",
    "table-horz",
    "underline-sigil",
];

const DECLARED_PRIMITIVES: &[&str] = &[
    "eof",
    "matching-codeblock-sigil",
    "repl-alphanumeric",
    "repl-any-character",
    "repl-carriage-return",
    "repl-digit1",
    "repl-eof",
    "repl-line-feed",
    "repl-load-path",
    "repl-non-space-line-ending-character",
    "repl-nonempty-through-line-ending",
    "repl-path-character",
    "repl-space0",
    "repl-space1",
    "repl-through-crlf",
];

// These names occur in the Rust-derived child graph but do not denote formal
// grammar edges at every call site:
//
// - `map`, `tag`, and `tuple` are nom combinators whose Rust identifiers
//   collide with inventoried grammar rules;
// - `grammar-peek` is parser control used to choose a parser without consuming;
// - the remaining names are embedded-parser or paragraph implementation calls
//   which the canonical grammar describes as prose mechanics at their call
//   sites.
const DECLARED_IMPLEMENTATION_ONLY_CHILDREN: &[&str] = &[
    "grammar-peek",
    "inline-paragraph",
    "map",
    "mech-code",
    "parse-grammar",
    "tag",
    "tuple",
];

#[derive(Debug)]
struct InventoryRow {
    line: usize,
    id: String,
    grammar_name: String,
    module: String,
    rust_function: String,
    classification: String,
    feature_gate: String,
    child_rules: String,
    implementation_path: String,
}

fn inventory_rows() -> Vec<InventoryRow> {
    let source = fs::read_to_string(INVENTORY_PATH)
        .unwrap_or_else(|err| panic!("failed to read {INVENTORY_PATH}: {err}"));
    let mut lines = source.lines();
    let header = lines
        .next()
        .unwrap_or_else(|| panic!("{INVENTORY_PATH} is empty"))
        .split('\t')
        .collect::<Vec<_>>();

    let index = |name: &str| {
        header
            .iter()
            .position(|column| *column == name)
            .unwrap_or_else(|| panic!("{INVENTORY_PATH} is missing column {name:?}"))
    };
    let id = index("id");
    let grammar_name = index("grammar-name");
    let module = index("module");
    let rust_function = index("rust-function");
    let classification = index("classification");
    let feature_gate = index("feature-gate");
    let child_rules = index("child-rules");
    let implementation_path = index("implementation-path");

    lines
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(offset, line)| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(
                fields.len(),
                header.len(),
                "{INVENTORY_PATH}:{} has {} fields; expected {}",
                offset + 2,
                fields.len(),
                header.len(),
            );
            InventoryRow {
                line: offset + 2,
                id: fields[id].to_owned(),
                grammar_name: fields[grammar_name].to_owned(),
                module: fields[module].to_owned(),
                rust_function: fields[rust_function].to_owned(),
                classification: fields[classification].to_owned(),
                feature_gate: fields[feature_gate].to_owned(),
                child_rules: fields[child_rules].to_owned(),
                implementation_path: fields[implementation_path].to_owned(),
            }
        })
        .collect()
}

fn is_rule_name(name: &str) -> bool {
    let mut chars = name.chars();
    if !matches!(chars.next(), Some('a'..='z')) {
        return false;
    }
    let mut previous_hyphen = false;
    for ch in chars {
        if ch == '-' {
            if previous_hyphen {
                return false;
            }
            previous_hyphen = true;
        } else if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            previous_hyphen = false;
        } else {
            return false;
        }
    }
    !previous_hyphen
}

/// This deliberately is not an EBNF parser. It only finds the stable
/// `kebab-name := ... ;` declarations inside the one inert canonical fence.
fn canonical_rules() -> BTreeMap<String, String> {
    let source = fs::read_to_string(SPECIFICATION_PATH)
        .unwrap_or_else(|err| panic!("failed to read {SPECIFICATION_PATH}: {err}"));
    assert_eq!(
        nested_ebnf_examples(&source),
        EXPECTED_NESTED_EBNF_EXAMPLES,
        "{SPECIFICATION_PATH}: the two literal nested EBNF fence examples must be preserved"
    );
    canonical_rules_from(&source)
}

fn nested_ebnf_examples(source: &str) -> usize {
    let mut in_tilde_fence = false;
    let mut examples = 0;

    for line in source.lines() {
        if line == "~~~" {
            in_tilde_fence = !in_tilde_fence;
        } else if in_tilde_fence && line.starts_with("```ebnf") {
            examples += 1;
        }
    }

    assert!(
        !in_tilde_fence,
        "{SPECIFICATION_PATH}: unclosed tilde fence"
    );
    examples
}

fn canonical_rules_from(source: &str) -> BTreeMap<String, String> {
    let mut rules = BTreeMap::new();
    let mut in_canonical_fence = false;
    let mut in_tilde_fence = false;
    let mut canonical_fences = 0;
    let mut current: Option<(String, usize, String)> = None;

    for (offset, line) in source.lines().enumerate() {
        let line_number = offset + 1;

        if !in_canonical_fence && line == "~~~" {
            in_tilde_fence = !in_tilde_fence;
            continue;
        }
        if in_tilde_fence {
            continue;
        }
        assert_ne!(
            line, "```ebnf",
            "{SPECIFICATION_PATH}:{line_number}: competing top-level executable EBNF fence"
        );
        if line == "```ebnf:canonical" {
            assert!(
                !in_canonical_fence,
                "{SPECIFICATION_PATH}:{line_number}: nested canonical grammar fence"
            );
            canonical_fences += 1;
            assert_eq!(
                canonical_fences, 1,
                "{SPECIFICATION_PATH}:{line_number}: multiple canonical grammar fences"
            );
            in_canonical_fence = true;
            continue;
        }
        if in_canonical_fence && line == "```" {
            assert!(
                current.is_none(),
                "{SPECIFICATION_PATH}:{line_number}: canonical grammar fence closed inside a rule"
            );
            in_canonical_fence = false;
            continue;
        }
        if !in_canonical_fence {
            continue;
        }

        if let Some((_, _, body)) = current.as_mut() {
            body.push(' ');
            body.push_str(line.trim());
            if line.trim_end().ends_with(';') {
                let (name, start, body) = current.take().unwrap();
                assert!(
                    rules.insert(name.clone(), body).is_none(),
                    "{SPECIFICATION_PATH}:{start}: duplicate canonical rule {name:?}"
                );
            }
            continue;
        }

        if line.trim().is_empty() {
            continue;
        }
        assert!(
            !line.starts_with(char::is_whitespace),
            "{SPECIFICATION_PATH}:{line_number}: indented canonical grammar line has no active declaration"
        );
        let Some((lhs, rhs)) = line.split_once(":=") else {
            panic!(
                "{SPECIFICATION_PATH}:{line_number}: unindented canonical grammar line is not a rule declaration"
            );
        };
        let name = lhs.trim();
        assert!(
            is_rule_name(name),
            "{SPECIFICATION_PATH}:{line_number}: canonical rule name {name:?} is not kebab-case"
        );
        current = Some((name.to_owned(), line_number, rhs.trim().to_owned()));
        if line.trim_end().ends_with(';') {
            let (name, start, body) = current.take().unwrap();
            assert!(
                rules.insert(name.clone(), body).is_none(),
                "{SPECIFICATION_PATH}:{start}: duplicate canonical rule {name:?}"
            );
        }
    }

    assert!(
        !in_tilde_fence,
        "{SPECIFICATION_PATH}: unclosed tilde fence"
    );
    assert!(
        !in_canonical_fence,
        "{SPECIFICATION_PATH}: unclosed canonical grammar fence"
    );
    assert!(
        current.is_none(),
        "{SPECIFICATION_PATH}: unterminated canonical grammar rule"
    );
    assert_eq!(
        canonical_fences, 1,
        "{SPECIFICATION_PATH}: expected exactly one canonical grammar fence"
    );
    rules
}

fn classification_has(classification: &str, role: &str) -> bool {
    classification.split('/').any(|part| part == role)
}

fn grammar_bearing(classification: &str) -> bool {
    ["root", "production", "terminal", "lexical-primitive"]
        .iter()
        .any(|role| classification_has(classification, role))
}

fn comma_names(field: &str) -> impl Iterator<Item = &str> {
    field
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty() && *name != "none")
}

fn unquoted_ascii_words(body: &str) -> Vec<String> {
    let trimmed = body.trim();
    let before_semicolon = trimmed.strip_suffix(';').unwrap_or(trimmed).trim();
    // A whole-rule `?descriptive implementation primitive?` has no graph edge.
    if before_semicolon.starts_with('?') && before_semicolon.ends_with('?') {
        return vec![];
    }

    let mut visible = String::with_capacity(body.len());
    let mut quoted = false;
    let mut escaped = false;
    for ch in body.chars() {
        if quoted {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            visible.push(' ');
        } else if ch == '"' {
            quoted = true;
            visible.push(' ');
        } else {
            visible.push(ch);
        }
    }

    visible
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
        .filter(|word| word.chars().any(|ch| ch.is_ascii_alphabetic()))
        .map(|word| {
            assert!(
                is_rule_name(word),
                "{SPECIFICATION_PATH}: malformed unquoted grammar name {word:?}"
            );
            word.to_owned()
        })
        .collect()
}

fn workspace_path(implementation_path: &str) -> PathBuf {
    let relative = implementation_path
        .split("::")
        .next()
        .unwrap_or(implementation_path);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn recursive_components(graph: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    fn visit(
        node: &str,
        graph: &BTreeMap<String, BTreeSet<String>>,
        seen: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) {
        if !seen.insert(node.to_owned()) {
            return;
        }
        for child in graph.get(node).into_iter().flatten() {
            visit(child, graph, seen, order);
        }
        order.push(node.to_owned());
    }

    let mut seen = BTreeSet::new();
    let mut order = vec![];
    for node in graph.keys() {
        visit(node, graph, &mut seen, &mut order);
    }

    let mut reverse: BTreeMap<String, BTreeSet<String>> = graph
        .keys()
        .map(|node| (node.clone(), BTreeSet::new()))
        .collect();
    for (parent, children) in graph {
        for child in children {
            reverse
                .entry(child.clone())
                .or_default()
                .insert(parent.clone());
        }
    }

    let mut assigned = BTreeSet::new();
    let mut recursive = vec![];
    while let Some(node) = order.pop() {
        if assigned.contains(&node) {
            continue;
        }
        let mut component = vec![];
        let mut queue = vec![node];
        while let Some(next) = queue.pop() {
            if !assigned.insert(next.clone()) {
                continue;
            }
            component.push(next.clone());
            queue.extend(reverse.get(&next).into_iter().flatten().cloned());
        }
        component.sort();
        let self_recursive = component.len() == 1
            && graph
                .get(&component[0])
                .is_some_and(|children| children.contains(&component[0]));
        if component.len() > 1 || self_recursive {
            recursive.push(component);
        }
    }
    recursive
}

#[test]
fn canonical_grammar_and_inventory_form_a_closed_rule_graph() {
    let rows = inventory_rows();
    let rules = canonical_rules();
    let declared_primitives = DECLARED_PRIMITIVES.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(
        rules.len(),
        EXPECTED_CANONICAL_RULES,
        "{SPECIFICATION_PATH}: canonical grammar rule count changed"
    );

    let mut manifest_names: BTreeMap<String, Vec<&InventoryRow>> = BTreeMap::new();
    let mut module_functions = BTreeSet::new();
    for row in &rows {
        assert!(
            module_functions.insert((row.module.as_str(), row.rust_function.as_str())),
            "{INVENTORY_PATH}:{} duplicates {}::{}",
            row.line,
            row.module,
            row.rust_function,
        );
        assert!(
            !row.id.is_empty(),
            "{INVENTORY_PATH}:{} has no stable id",
            row.line
        );
        assert!(
            !row.feature_gate.is_empty(),
            "{INVENTORY_PATH}:{} has no feature-gate classification",
            row.line,
        );
        if grammar_bearing(&row.classification) {
            assert!(
                !row.grammar_name.is_empty(),
                "{INVENTORY_PATH}:{} grammar-bearing {}::{} has no grammar name",
                row.line,
                row.module,
                row.rust_function,
            );
        }
        if !row.grammar_name.is_empty() {
            assert!(
                is_rule_name(&row.grammar_name),
                "{INVENTORY_PATH}:{} has invalid grammar name {:?}",
                row.line,
                row.grammar_name,
            );
            assert!(
                !row.implementation_path.is_empty(),
                "{INVENTORY_PATH}:{} rule {:?} has no implementation path",
                row.line,
                row.grammar_name,
            );
            assert!(
                workspace_path(&row.implementation_path).is_file(),
                "{INVENTORY_PATH}:{} implementation path {:?} does not name a file",
                row.line,
                row.implementation_path,
            );
            manifest_names
                .entry(row.grammar_name.clone())
                .or_default()
                .push(row);
        }
    }

    let defined = rules.keys().cloned().collect::<BTreeSet<_>>();
    let inventoried = manifest_names.keys().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        inventoried.len(),
        EXPECTED_CANONICAL_RULES,
        "{INVENTORY_PATH}: inventoried grammar rule count changed"
    );
    assert_eq!(
        defined, inventoried,
        "specification rules and inventoried canonical grammar names differ"
    );

    // `child-rules` is the manifest-driven implementation graph. It must never
    // point at an undeclared production.
    for row in &rows {
        for child in comma_names(&row.child_rules) {
            assert!(
                inventoried.contains(child) || declared_primitives.contains(child),
                "{INVENTORY_PATH}:{} rule {:?} references undefined child {:?}",
                row.line,
                row.grammar_name,
                child,
            );
        }
    }

    // Scan only names from the inert formal rule bodies. This catches canonical
    // transcription typos while intentionally avoiding a generalized EBNF AST.
    let mut graph: BTreeMap<String, BTreeSet<String>> = rules
        .keys()
        .map(|name| (name.clone(), BTreeSet::new()))
        .collect();
    for (name, body) in &rules {
        for reference in unquoted_ascii_words(body) {
            if defined.contains(&reference) {
                graph.get_mut(name).unwrap().insert(reference);
            } else {
                assert!(
                    declared_primitives.contains(reference.as_str()),
                    "{SPECIFICATION_PATH}: rule {name:?} references undefined name {reference:?}",
                );
            }
        }
    }

    // The inventory graph is Rust-call-shaped while the formal graph may inline
    // wrappers or spell combinators as operators. Nevertheless, every named
    // language child recorded by the inventory must be reachable from the
    // corresponding formal rule. This one-way cross-check catches transcription
    // drift (for example `right-angle1` versus the implemented `right-angle`)
    // without pretending the two graphs have identical abstraction boundaries.
    // It is deliberately a coarse reachability guard, not proof of direct-edge
    // equivalence: formal-only edges and alternate paths inside recursive
    // components remain review responsibilities.
    let implementation_only = DECLARED_IMPLEMENTATION_ONLY_CHILDREN
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let edge_drift = rows
        .iter()
        .filter(|row| !row.grammar_name.is_empty())
        .flat_map(|row| {
            let mut reachable = BTreeSet::new();
            let mut queue =
                VecDeque::from_iter(graph.get(&row.grammar_name).into_iter().flatten().cloned());
            while let Some(rule) = queue.pop_front() {
                if !reachable.insert(rule.clone()) {
                    continue;
                }
                queue.extend(graph.get(&rule).into_iter().flatten().cloned());
            }
            comma_names(&row.child_rules)
                .filter(|child| {
                    defined.contains(*child)
                        && !implementation_only.contains(*child)
                        && !reachable.contains(*child)
                })
                .map(|child| (row.line, row.grammar_name.clone(), child.to_owned()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert!(
        edge_drift.is_empty(),
        "inventory language children missing from canonical reachability: {edge_drift:#?}"
    );

    let roots = rows
        .iter()
        .filter(|row| classification_has(&row.classification, "root"))
        .map(|row| row.grammar_name.clone())
        .collect::<BTreeSet<_>>();
    let mut reachable = BTreeSet::new();
    let mut queue = VecDeque::from_iter(roots.iter().cloned());
    while let Some(rule) = queue.pop_front() {
        if !reachable.insert(rule.clone()) {
            continue;
        }
        queue.extend(graph.get(&rule).into_iter().flatten().cloned());
    }

    let unreachable = defined
        .difference(&reachable)
        .cloned()
        .collect::<BTreeSet<_>>();
    let classified_unreachable = DECLARED_SEPARATE_ROOTS
        .iter()
        .chain(DECLARED_LEGACY_OR_DEAD)
        .map(|name| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        unreachable, classified_unreachable,
        "unreachable rules must be declared as a separate root or legacy/dead"
    );

    let cycles = recursive_components(&graph);
    let cycle_sizes = cycles.iter().map(Vec::len).collect::<Vec<_>>();
    eprintln!(
        "grammar rule graph: rules={}, primitives={}, roots={}, feature-gated={}, \
     undefined=0, duplicate-definitions=0, unmapped-canonical=0, \
     unclassified-implementation=0, recursive-components={} {:?}",
        rules.len(),
        rows.iter()
            .filter(|row| classification_has(&row.classification, "lexical-primitive"))
            .count(),
        roots.len(),
        rows.iter()
            .filter(|row| row.feature_gate != "always")
            .count(),
        cycles.len(),
        cycle_sizes,
    );
}

#[test]
fn canonical_grammar_is_inert_parseable_specification() {
    let source = fs::read_to_string(SPECIFICATION_PATH)
        .unwrap_or_else(|err| panic!("failed to read {SPECIFICATION_PATH}: {err}"));
    mech_syntax::parse(&source).unwrap_or_else(|err| {
        panic!("{SPECIFICATION_PATH} must parse with its canonical grammar inert: {err:?}")
    });
}

#[test]
fn canonical_scanner_rejects_malformed_declarations_and_references() {
    let indented = "```ebnf:canonical\n  bad-name := \"x\" ;\n```\n";
    assert!(
        std::panic::catch_unwind(|| canonical_rules_from(indented)).is_err(),
        "an orphaned indented declaration must not disappear from the rule graph"
    );

    let competing = "```ebnf\nold := \"x\" ;\n```\n```ebnf:canonical\ncanonical := \"x\" ;\n```\n";
    assert!(
        std::panic::catch_unwind(|| canonical_rules_from(competing)).is_err(),
        "a competing top-level executable EBNF block must be rejected"
    );

    assert!(
        std::panic::catch_unwind(|| unquoted_ascii_words("BAR ;")).is_err(),
        "an uppercase bare reference must not disappear from the rule graph"
    );
    assert!(!is_rule_name("bad_name"));
    assert!(!is_rule_name("bad--name"));
    assert!(!is_rule_name("bad-name-"));
}
