use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

const EXPECTED_RULES: usize = 539;
const EXPECTED_PHASE_2I_RULES: usize = 80;
const EXPECTED_PHASE_2I_COMPONENTS: usize = 1;
const DEPENDENCY_HEADER: &str = "grammar-name\tdirect-children\tdirect-parents";
const SCC_HEADER: &str = "component-id\tcomponent-size\trecursive\tmembers\t\
                          outgoing-unported-components\toutgoing-ported-rules";
const PHASE_HEADER: &str = "grammar-name\tfamily\tcomponent-id\tcomponent-size\t\
                            recursive-component\tsame-component-children\tclosure-children\t\
                            ported-external-children";

const ANCHORS: &[&str] = &[
    "expression",
    "formula",
    "factor",
    "literal",
    "kind-annotation",
    "kind",
    "kind-scalar",
    "var",
    "subscript",
    "slice",
    "structure",
    "matrix",
    "map",
    "set",
    "tuple",
    "function-call",
    "pattern",
    "comprehension-qualifier",
    "variable-define",
    "fsm-pipe",
];

#[derive(Clone, Debug)]
struct Port {
    family: String,
    syntax: String,
    lowering: String,
    policy: String,
    phase: String,
}

#[derive(Clone, Debug)]
struct SccRow {
    id: String,
    size: usize,
    recursive: bool,
    members: BTreeSet<String>,
    outgoing_unported: BTreeSet<String>,
    outgoing_ported: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct PhaseRow {
    family: String,
    component_id: String,
    component_size: usize,
    recursive: bool,
    same_component: BTreeSet<String>,
    closure_children: BTreeSet<String>,
    ported_external: BTreeSet<String>,
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
    assert!(!names.contains(""), "list contains an empty name");
    assert_eq!(names.len(), field.split('|').count(), "duplicate list name");
    names
}

fn parse_bool(field: &str) -> bool {
    match field {
        "true" => true,
        "false" => false,
        other => panic!("invalid boolean {other}"),
    }
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

fn dependencies() -> BTreeMap<String, BTreeSet<String>> {
    let source = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/canonical-dependencies.tsv"),
    )
    .expect("read canonical-dependencies.tsv");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(DEPENDENCY_HEADER));
    let mut previous = String::new();
    let mut graph = BTreeMap::new();
    let mut reported_parents = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        assert!(!line.is_empty(), "blank dependency row {}", index + 2);
        let row = fields(line);
        assert_eq!(row.len(), 3, "invalid dependency row {}", index + 2);
        assert!(
            row[0] > previous.as_str(),
            "dependency rows are not ordered"
        );
        previous = row[0].to_owned();
        assert!(
            graph
                .insert(row[0].to_owned(), parse_name_list(row[1]))
                .is_none()
        );
        reported_parents.insert(row[0].to_owned(), parse_name_list(row[2]));
    }
    assert_eq!(graph.len(), EXPECTED_RULES);
    let mut reversed = graph
        .keys()
        .map(|name| (name.clone(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for (parent, children) in &graph {
        for child in children {
            reversed
                .get_mut(child)
                .unwrap_or_else(|| panic!("unknown dependency {child}"))
                .insert(parent.clone());
        }
    }
    assert_eq!(reported_parents, reversed);
    graph
}

fn ports() -> BTreeMap<String, Port> {
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
    let mut previous = String::new();
    let mut ports = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        assert!(!line.is_empty(), "blank port row {}", index + 2);
        let row = fields(line);
        assert_eq!(row.len(), 7, "invalid port row {}", index + 2);
        assert!(row[0] > previous.as_str(), "ports are not ordered");
        previous = row[0].to_owned();
        assert!(
            ports
                .insert(
                    row[0].to_owned(),
                    Port {
                        family: row[1].to_owned(),
                        syntax: row[2].to_owned(),
                        lowering: row[3].to_owned(),
                        policy: row[4].to_owned(),
                        phase: row[5].to_owned(),
                    },
                )
                .is_none()
        );
    }
    assert_eq!(ports.len(), EXPECTED_RULES);
    ports
}

fn scc_report() -> BTreeMap<String, SccRow> {
    let source =
        fs::read_to_string(repository_root().join("docs/design/grammar-audit/unported-sccs.tsv"))
            .expect("read unported-sccs.tsv");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(SCC_HEADER));
    let mut rows = BTreeMap::new();
    let mut all_members = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        assert!(!line.is_empty(), "blank SCC row {}", index + 2);
        let row = fields(line);
        assert_eq!(row.len(), 6, "invalid SCC row {}", index + 2);
        let members = parse_name_list(row[3]);
        let size = row[1].parse::<usize>().expect("component size");
        assert_eq!(size, members.len(), "component size for {}", row[0]);
        for member in &members {
            assert!(
                all_members.insert(member.clone()),
                "duplicate SCC member {member}"
            );
        }
        let value = SccRow {
            id: row[0].to_owned(),
            size,
            recursive: parse_bool(row[2]),
            members,
            outgoing_unported: parse_name_list(row[4]),
            outgoing_ported: parse_name_list(row[5]),
        };
        assert!(rows.insert(row[0].to_owned(), value).is_none());
    }
    rows
}

fn phase_report() -> BTreeMap<String, PhaseRow> {
    let source = fs::read_to_string(
        repository_root().join("docs/design/grammar-audit/phase-2i-recursive-core.tsv"),
    )
    .expect("read phase-2i-recursive-core.tsv");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(PHASE_HEADER));
    let mut previous = String::new();
    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        assert!(!line.is_empty(), "blank Phase 2I row {}", index + 2);
        let row = fields(line);
        assert_eq!(row.len(), 8, "invalid Phase 2I row {}", index + 2);
        assert!(row[0] > previous.as_str(), "Phase 2I rows are not ordered");
        previous = row[0].to_owned();
        let value = PhaseRow {
            family: row[1].to_owned(),
            component_id: row[2].to_owned(),
            component_size: row[3].parse().expect("Phase 2I component size"),
            recursive: parse_bool(row[4]),
            same_component: parse_name_list(row[5]),
            closure_children: parse_name_list(row[6]),
            ported_external: parse_name_list(row[7]),
        };
        assert!(rows.insert(row[0].to_owned(), value).is_none());
    }
    rows
}

fn visit(
    node: &str,
    graph: &BTreeMap<String, BTreeSet<String>>,
    visited: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) {
    if !visited.insert(node.to_owned()) {
        return;
    }
    for child in &graph[node] {
        visit(child, graph, visited, order);
    }
    order.push(node.to_owned());
}

fn collect_component(
    node: &str,
    reversed: &BTreeMap<String, BTreeSet<String>>,
    visited: &mut BTreeSet<String>,
    members: &mut BTreeSet<String>,
) {
    if !visited.insert(node.to_owned()) {
        return;
    }
    members.insert(node.to_owned());
    for parent in &reversed[node] {
        collect_component(parent, reversed, visited, members);
    }
}

fn components(graph: &BTreeMap<String, BTreeSet<String>>) -> Vec<BTreeSet<String>> {
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for node in graph.keys() {
        visit(node, graph, &mut visited, &mut order);
    }
    let mut reversed = graph
        .keys()
        .map(|name| (name.clone(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for (parent, children) in graph {
        for child in children {
            reversed.get_mut(child).unwrap().insert(parent.clone());
        }
    }
    visited.clear();
    let mut components = Vec::new();
    for node in order.into_iter().rev() {
        if visited.contains(&node) {
            continue;
        }
        let mut members = BTreeSet::new();
        collect_component(&node, &reversed, &mut visited, &mut members);
        components.push(members);
    }
    components.sort_by(|left, right| {
        right
            .len()
            .cmp(&left.len())
            .then_with(|| left.iter().cmp(right.iter()))
    });
    components
}

fn component_by_rule(components: &[BTreeSet<String>]) -> BTreeMap<String, usize> {
    components
        .iter()
        .enumerate()
        .flat_map(|(index, component)| component.iter().cloned().map(move |member| (member, index)))
        .collect()
}

fn is_unported(port: &Port) -> bool {
    port.syntax == "unported"
}

fn is_implemented(port: &Port) -> bool {
    matches!(port.syntax.as_str(), "syntax-ported" | "parity-verified")
}

fn ordinary_closure(
    graph: &BTreeMap<String, BTreeSet<String>>,
    ports: &BTreeMap<String, Port>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut closure = BTreeSet::new();
    let mut external = BTreeSet::new();
    let mut pending = vec!["expression".to_owned()];
    while let Some(node) = pending.pop() {
        if is_implemented(&ports[&node]) {
            external.insert(node);
            continue;
        }
        assert!(
            is_unported(&ports[&node]),
            "invalid syntax status for {node}"
        );
        if !closure.insert(node.clone()) {
            continue;
        }
        pending.extend(graph[&node].iter().cloned());
    }
    (closure, external)
}

#[test]
fn report_schemas_ordering_and_uniqueness_are_exact() {
    let graph = dependencies();
    let ports = ports();
    let inventory = canonical_inventory_names();
    assert_eq!(graph.keys().cloned().collect::<BTreeSet<_>>(), inventory);
    assert_eq!(ports.keys().cloned().collect::<BTreeSet<_>>(), inventory);

    let sccs = scc_report();
    let phase = phase_report();
    assert_eq!(phase.len(), EXPECTED_PHASE_2I_RULES);
    let phase_components = phase
        .values()
        .map(|row| row.component_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(phase_components.len(), EXPECTED_PHASE_2I_COMPONENTS);
    for (name, row) in &phase {
        let component = &sccs[&row.component_id];
        assert!(component.members.contains(name));
        assert_eq!(row.component_size, component.size);
        assert_eq!(row.recursive, component.recursive);
    }
}

#[test]
fn kosaraju_independently_recomputes_every_unported_component() {
    let graph = dependencies();
    let ports = ports();
    let components = components(&graph);
    let by_rule = component_by_rule(&components);
    let report = scc_report();
    let reported_by_member = report
        .values()
        .flat_map(|row| row.members.iter().cloned().map(move |member| (member, row)))
        .collect::<BTreeMap<_, _>>();

    for component in &components {
        let unported = component
            .iter()
            .filter(|member| is_unported(&ports[*member]))
            .count();
        assert!(
            unported == 0 || unported == component.len(),
            "mixed port-status SCC: {component:?}"
        );
    }

    let unported_names = ports
        .iter()
        .filter_map(|(name, port)| is_unported(port).then_some(name.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        reported_by_member.keys().cloned().collect::<BTreeSet<_>>(),
        unported_names
    );

    let reported_id_by_component = report
        .values()
        .map(|row| (row.members.clone(), row.id.clone()))
        .collect::<BTreeMap<_, _>>();
    for name in &unported_names {
        let component = &components[by_rule[name]];
        let row = reported_by_member[name];
        assert_eq!(&row.members, component, "SCC membership for {name}");
        let recursive = component.len() > 1 || graph[name].contains(name);
        assert_eq!(row.recursive, recursive, "recursive flag for {name}");
    }

    for row in report.values() {
        let mut outgoing_unported = BTreeSet::new();
        let mut outgoing_ported = BTreeSet::new();
        for member in &row.members {
            for child in &graph[member] {
                if row.members.contains(child) {
                    continue;
                }
                if is_unported(&ports[child]) {
                    let target = &components[by_rule[child]];
                    outgoing_unported.insert(reported_id_by_component[target].clone());
                } else {
                    assert!(is_implemented(&ports[child]));
                    outgoing_ported.insert(child.clone());
                }
            }
        }
        assert_eq!(
            row.outgoing_unported, outgoing_unported,
            "outgoing SCCs for {}",
            row.id
        );
        assert_eq!(
            row.outgoing_ported, outgoing_ported,
            "ported dependencies for {}",
            row.id
        );
    }
}

#[test]
fn expression_root_closure_is_exact_and_dependency_closed() {
    let graph = dependencies();
    let ports = ports();
    let phase = phase_report();
    let phase_names = phase.keys().cloned().collect::<BTreeSet<_>>();
    let (closure, external) = ordinary_closure(&graph, &ports);
    assert_eq!(closure, phase_names);

    let components = components(&graph);
    let by_rule = component_by_rule(&components);
    let root_component = by_rule["expression"];
    let mut included_components = BTreeSet::new();
    let mut pending = vec![root_component];
    while let Some(index) = pending.pop() {
        if !included_components.insert(index) {
            continue;
        }
        assert!(
            components[index]
                .iter()
                .all(|name| is_unported(&ports[name]))
        );
        for member in &components[index] {
            for child in &graph[member] {
                let target = by_rule[child];
                if target != index && is_unported(&ports[child]) {
                    pending.push(target);
                }
            }
        }
    }
    let scc_closure = included_components
        .iter()
        .flat_map(|index| components[*index].iter().cloned())
        .collect::<BTreeSet<_>>();
    assert_eq!(scc_closure, phase_names);
    assert_eq!(included_components.len(), EXPECTED_PHASE_2I_COMPONENTS);

    let reported_external = phase
        .values()
        .flat_map(|row| row.ported_external.iter().cloned())
        .collect::<BTreeSet<_>>();
    assert_eq!(external, reported_external);
    assert_eq!(external.len(), 74);
}

#[test]
fn phase_boundary_rows_preserve_all_port_and_edge_invariants() {
    let graph = dependencies();
    let ports = ports();
    let phase = phase_report();
    let phase_names = phase.keys().cloned().collect::<BTreeSet<_>>();
    let by_component = phase.iter().fold(
        BTreeMap::<String, BTreeSet<String>>::new(),
        |mut grouped, (name, row)| {
            grouped
                .entry(row.component_id.clone())
                .or_default()
                .insert(name.clone());
            grouped
        },
    );

    for (name, row) in &phase {
        let port = &ports[name];
        assert_eq!(row.family, port.family);
        assert!(is_unported(port));
        assert!(port.phase.is_empty());
        assert_eq!(port.policy, "undecided");
        assert_eq!(port.lowering, "pending");

        let mut same_component = BTreeSet::new();
        let mut closure_children = BTreeSet::new();
        let mut ported_external = BTreeSet::new();
        for child in &graph[name] {
            if by_component[&row.component_id].contains(child) {
                same_component.insert(child.clone());
            } else if phase_names.contains(child) {
                closure_children.insert(child.clone());
            } else {
                assert!(
                    is_implemented(&ports[child]),
                    "unported outgoing child {child}"
                );
                ported_external.insert(child.clone());
            }
        }
        assert_eq!(
            row.same_component, same_component,
            "same-component children for {name}"
        );
        assert_eq!(
            row.closure_children, closure_children,
            "closure children for {name}"
        );
        assert_eq!(
            row.ported_external, ported_external,
            "external children for {name}"
        );
    }
}

#[test]
fn reviewed_count_anchors_and_families_are_frozen() {
    let phase = phase_report();
    assert_eq!(phase.len(), EXPECTED_PHASE_2I_RULES);
    let components = phase
        .values()
        .map(|row| row.component_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(components.len(), EXPECTED_PHASE_2I_COMPONENTS);
    for anchor in ANCHORS {
        assert!(
            phase.contains_key(*anchor),
            "missing Phase 2I anchor {anchor}"
        );
    }
    for (name, row) in &phase {
        assert!(
            !matches!(
                row.family.as_str(),
                "mechdown" | "mika" | "repl" | "activation" | "parser"
            ),
            "forbidden Phase 2I family for {name}: {}",
            row.family
        );
    }
}

#[test]
fn recursive_core_has_the_exact_implementation_shell_and_remains_unactivated() {
    let root = repository_root();
    let parser_directory = root.join("src/syntax/src/document/parser/canonical/recursive_core");
    assert!(parser_directory.is_dir());
    let actual_files = fs::read_dir(&parser_directory)
        .expect("read recursive parser directory")
        .map(|entry| {
            entry
                .expect("read recursive parser entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<BTreeSet<_>>();
    let expected_files = [
        "calls.rs",
        "comprehensions.rs",
        "expressions.rs",
        "fsm.rs",
        "kinds.rs",
        "literals.rs",
        "mod.rs",
        "patterns.rs",
        "precedence.rs",
        "structures.rs",
        "subscripts.rs",
        "variables.rs",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    assert_eq!(actual_files, expected_files);

    for path in [
        "src/syntax/src/document/ast/recursive_core",
        "src/syntax/src/document/lower/legacy/recursive_core",
    ] {
        assert!(
            !root.join(path).exists(),
            "Phase 2I implementation path exists: {path}"
        );
    }
    let generated_ports =
        fs::read_to_string(root.join("src/syntax/src/document/parser/canonical_ports.rs"))
            .expect("read canonical_ports.rs");
    assert!(!generated_ports.contains("Phase2I"));

    let phase = phase_report();
    let ports = ports();
    assert_eq!(phase.len(), EXPECTED_PHASE_2I_RULES);
    for name in phase.keys() {
        let port = &ports[name];
        assert!(is_unported(port), "{name}");
        assert_eq!(port.lowering, "pending", "{name}");
        assert_eq!(port.policy, "undecided", "{name}");
        assert!(port.phase.is_empty(), "{name}");
    }
}

#[test]
fn recursive_dispatcher_and_production_functions_are_exactly_the_frozen_eighty() {
    let root = repository_root();
    let parser_directory = root.join("src/syntax/src/document/parser/canonical/recursive_core");
    let expected_functions = phase_report()
        .keys()
        .map(|name| format!("parse_{}", name.replace('-', "_")))
        .collect::<BTreeSet<_>>();
    assert_eq!(expected_functions.len(), EXPECTED_PHASE_2I_RULES);

    let mut actual_functions = BTreeSet::new();
    for entry in fs::read_dir(&parser_directory).expect("read recursive parser directory") {
        let path = entry.expect("read recursive parser entry").path();
        if path.file_name().and_then(|name| name.to_str()) == Some("mod.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read recursive production module");
        for suffix in source.split("pub(super) fn ").skip(1) {
            let name = suffix.split('(').next().expect("function name").trim();
            if name.starts_with("parse_") {
                assert!(
                    actual_functions.insert(name.to_owned()),
                    "duplicate production function {name}"
                );
            }
        }
    }
    assert_eq!(actual_functions, expected_functions);

    let module = fs::read_to_string(parser_directory.join("mod.rs")).expect("read dispatcher");
    let inventory_start = module.find("pub(crate) const PHASE_2I_RULES").unwrap();
    let inventory_end = module[inventory_start..].find("];\n").unwrap() + inventory_start;
    let inventory = &module[inventory_start..inventory_end];
    assert_eq!(
        inventory.matches("rules::").count(),
        EXPECTED_PHASE_2I_RULES
    );

    let dispatcher_start = module.find("pub(crate) fn parse_rule").unwrap();
    let dispatcher = &module[dispatcher_start..];
    assert_eq!(
        dispatcher
            .lines()
            .filter(|line| line.contains("rules::") && line.contains("=>"))
            .count(),
        EXPECTED_PHASE_2I_RULES
    );
    assert!(dispatcher.contains("_ => return None"));
}
