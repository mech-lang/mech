//! Direct, crate-internal parity for the closed module-import productions.
//!
//! These checks compare each canonical source production with its corrected
//! legacy parser counterpart. They intentionally live beside the private
//! legacy functions so the contract does not create a new public parser API.

use super::*;

use crate::document::lower::legacy::{LegacyModuleImportValue, lower_phase_2e_module_import_value};
use crate::document::parser::canonical::imports::PHASE_2E_IMPORT_RULES;
use crate::document::parser::canonical::{
    CanonicalRuleOutcome, CanonicalSourceRuleSnapshot, parse_canonical_phase_2e_rule_for_test,
};
use crate::document::parser::rule::rules;
use crate::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange, TextSize,
    TextSnapshot, reconstruct_source_range, validate_lossless_range,
};
use crate::{ParseResult, ParseString};
use nom::Err;

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParityValue {
    Node(LegacyModuleImportValue),
    Transparent,
}

type LegacyParser = for<'source> fn(ParseString<'source>) -> ParseResult<'source, ParityValue>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Prefix {
    consumed: TextSize,
    remaining: TextSize,
}

#[derive(Debug, Eq, PartialEq)]
struct LegacyMatch {
    value: ParityValue,
    prefix: Prefix,
}

#[derive(Debug, Eq, PartialEq)]
enum LegacyOutcome {
    Matched(LegacyMatch),
    Error { reached: TextSize },
    Failure { reached: TextSize },
}

#[derive(Clone, Copy)]
struct Contract {
    name: &'static str,
    rule: RuleId,
    kind: Option<SyntaxKind>,
    parser: LegacyParser,
    probes: [&'static str; 5],
}

macro_rules! legacy_node_parser {
    ($name:ident, $parser:path, $variant:ident) => {
        fn $name<'source>(input: ParseString<'source>) -> ParseResult<'source, ParityValue> {
            let (input, value) = $parser(input)?;
            Ok((
                input,
                ParityValue::Node(LegacyModuleImportValue::$variant(value)),
            ))
        }
    };
}

legacy_node_parser!(
    legacy_module_import_name_segment,
    super::module_import_name_segment,
    NameSegment
);
legacy_node_parser!(
    legacy_module_import_intrinsic_segment,
    super::module_import_intrinsic_segment,
    IntrinsicSegment
);
legacy_node_parser!(
    legacy_module_import_path_segment,
    super::module_import_path_segment,
    PathSegment
);
legacy_node_parser!(legacy_module_import_path, super::module_import_path, Path);
legacy_node_parser!(
    legacy_module_import_alias_segment,
    super::module_import_alias_segment,
    AliasSegment
);
legacy_node_parser!(
    legacy_module_import_alias_path,
    super::module_import_alias_path,
    AliasPath
);
legacy_node_parser!(
    legacy_module_import_value_alias,
    super::module_import_value_alias,
    ValueAlias
);
legacy_node_parser!(
    legacy_context_import_alias_segment,
    super::context_import_alias_segment,
    ContextAliasSegment
);
legacy_node_parser!(
    legacy_module_import_context_alias,
    super::module_import_context_alias,
    ContextAlias
);
legacy_node_parser!(
    legacy_module_import_alias,
    super::module_import_alias,
    Alias
);
legacy_node_parser!(legacy_module_root, super::module_root, Root);
legacy_node_parser!(
    legacy_import_group_item,
    super::import_group_item,
    GroupItem
);
legacy_node_parser!(
    legacy_import_group_items,
    super::import_group_items,
    GroupItems
);
legacy_node_parser!(
    legacy_aliased_item_import,
    super::aliased_item_import,
    AliasedItem
);
legacy_node_parser!(
    legacy_module_suffix_import,
    super::module_suffix_import,
    Suffix
);
legacy_node_parser!(
    legacy_module_only_import,
    super::module_only_import,
    ModuleOnly
);
legacy_node_parser!(legacy_module_import, super::module_import, Import);

fn legacy_import_alias_operator<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, ParityValue> {
    let (input, ()) = super::import_alias_operator(input)?;
    Ok((input, ParityValue::Transparent))
}

fn legacy_import_group_separator<'source>(
    input: ParseString<'source>,
) -> ParseResult<'source, ParityValue> {
    let (input, ()) = super::import_group_separator(input)?;
    Ok((input, ParityValue::Transparent))
}

fn phase_2e_contracts() -> [Contract; 19] {
    [
        Contract {
            name: "module-import-name-segment",
            rule: rules::MODULE_IMPORT_NAME_SEGMENT,
            kind: Some(SyntaxKind::ModuleImportNameSegment),
            parser: legacy_module_import_name_segment,
            probes: ["math", "math2", "math/", "/", "math "],
        },
        Contract {
            name: "module-import-intrinsic-segment",
            rule: rules::MODULE_IMPORT_INTRINSIC_SEGMENT,
            kind: Some(SyntaxKind::ModuleImportIntrinsicSegment),
            parser: legacy_module_import_intrinsic_segment,
            probes: ["_math", "_math2", "_math/", "math", "_"],
        },
        Contract {
            name: "module-import-path-segment",
            rule: rules::MODULE_IMPORT_PATH_SEGMENT,
            kind: Some(SyntaxKind::ModuleImportPathSegment),
            parser: legacy_module_import_path_segment,
            probes: ["math", "_math", "math/", "/math", "_"],
        },
        Contract {
            name: "module-import-path",
            rule: rules::MODULE_IMPORT_PATH,
            kind: Some(SyntaxKind::ModuleImportPath),
            parser: legacy_module_import_path,
            probes: ["math", "math/trig/_sin", "math/", "/math", "math/_"],
        },
        Contract {
            name: "module-import-alias-segment",
            rule: rules::MODULE_IMPORT_ALIAS_SEGMENT,
            kind: Some(SyntaxKind::ModuleImportAliasSegment),
            parser: legacy_module_import_alias_segment,
            probes: ["alias", "alias2", "alias/", "/", "alias "],
        },
        Contract {
            name: "module-import-alias-path",
            rule: rules::MODULE_IMPORT_ALIAS_PATH,
            kind: Some(SyntaxKind::ModuleImportAliasPath),
            parser: legacy_module_import_alias_path,
            probes: ["alias", "alias/path", "alias/", "/alias", "alias/path "],
        },
        Contract {
            name: "module-import-value-alias",
            rule: rules::MODULE_IMPORT_VALUE_ALIAS,
            kind: Some(SyntaxKind::ModuleImportValueAlias),
            parser: legacy_module_import_value_alias,
            probes: ["alias", "alias/path", "alias/", "/alias", "alias/path "],
        },
        Contract {
            name: "context-import-alias-segment",
            rule: rules::CONTEXT_IMPORT_ALIAS_SEGMENT,
            kind: Some(SyntaxKind::ContextImportAliasSegment),
            parser: legacy_context_import_alias_segment,
            probes: ["ctx", "ctx-2", "ctx/", "2ctx", "ctx--"],
        },
        Contract {
            name: "module-import-context-alias",
            rule: rules::MODULE_IMPORT_CONTEXT_ALIAS,
            kind: Some(SyntaxKind::ModuleImportContextAlias),
            parser: legacy_module_import_context_alias,
            probes: ["@ctx", "@ctx-2", "@ctx/", "ctx", "@ctx!"],
        },
        Contract {
            name: "module-import-alias",
            rule: rules::MODULE_IMPORT_ALIAS,
            kind: Some(SyntaxKind::ModuleImportAlias),
            parser: legacy_module_import_alias,
            probes: ["@ctx", "alias/path", "alias/", "/", "alias "],
        },
        Contract {
            name: "module-root",
            rule: rules::MODULE_ROOT,
            kind: Some(SyntaxKind::ModuleRoot),
            parser: legacy_module_root,
            probes: ["math", "math2", "math/", "/", "math "],
        },
        Contract {
            name: "import-alias-operator",
            rule: rules::IMPORT_ALIAS_OPERATOR,
            kind: None,
            parser: legacy_import_alias_operator,
            probes: [":=", " := ", ":=tail", ":", " = "],
        },
        Contract {
            name: "import-group-separator",
            rule: rules::IMPORT_GROUP_SEPARATOR,
            kind: None,
            parser: legacy_import_group_separator,
            probes: [",", " \n", ",tail", "", ";"],
        },
        Contract {
            name: "import-group-item",
            rule: rules::IMPORT_GROUP_ITEM,
            kind: Some(SyntaxKind::ImportGroupItem),
            parser: legacy_import_group_item,
            probes: ["sin", "trig/sin", "sin/", "/", "_"],
        },
        Contract {
            name: "import-group-items",
            rule: rules::IMPORT_GROUP_ITEMS,
            kind: Some(SyntaxKind::ImportGroupItems),
            parser: legacy_import_group_items,
            probes: ["sin", "sin,cos", "sin ", " ", "sin\ncos"],
        },
        Contract {
            name: "aliased-item-import",
            rule: rules::ALIASED_ITEM_IMPORT,
            kind: Some(SyntaxKind::AliasedItemImport),
            parser: legacy_aliased_item_import,
            probes: [
                "alias := math/sin",
                "@ctx := math/sin",
                "alias := math/sin/",
                ":= math/sin",
                "alias :=",
            ],
        },
        Contract {
            name: "module-suffix-import",
            rule: rules::MODULE_SUFFIX_IMPORT,
            kind: Some(SyntaxKind::ModuleSuffixImport),
            parser: legacy_module_suffix_import,
            probes: ["math/*", "math/{sin,cos}", "math/sin/", "/math", "math/"],
        },
        Contract {
            name: "module-only-import",
            rule: rules::MODULE_ONLY_IMPORT,
            kind: Some(SyntaxKind::ModuleOnlyImport),
            parser: legacy_module_only_import,
            probes: ["math", "math2", "math ", "/math", "math/"],
        },
        Contract {
            name: "module-import",
            rule: rules::MODULE_IMPORT,
            kind: Some(SyntaxKind::ModuleImport),
            parser: legacy_module_import,
            probes: ["+>math", "+> math/sin", " +> math", "math", "+> math/"],
        },
    ]
}

fn legacy_outcome(input: &str, parser: LegacyParser) -> LegacyOutcome {
    let graphemes = crate::graphemes::init_tag(input);
    let to_offset = |cursor: usize| {
        assert!(
            cursor <= graphemes.len(),
            "legacy parser cursor {cursor} exceeds {} graphemes for {input:?}",
            graphemes.len(),
        );
        TextSize::from_u32(
            graphemes[..cursor]
                .iter()
                .map(|grapheme| grapheme.len() as u32)
                .sum(),
        )
    };
    let prefix = |cursor: usize| {
        let consumed = to_offset(cursor);
        Prefix {
            consumed,
            remaining: TextSize::from_u32(input.len() as u32) - consumed,
        }
    };

    match parser(ParseString::new(&graphemes)) {
        Ok((remaining, value)) => {
            assert!(
                remaining.error_log.is_empty(),
                "legacy parser recorded errors after accepting {input:?}: {:?}",
                remaining.error_log,
            );
            LegacyOutcome::Matched(LegacyMatch {
                value,
                prefix: prefix(remaining.cursor),
            })
        }
        Err(Err::Error(error)) => LegacyOutcome::Error {
            reached: to_offset(error.remaining_input.cursor),
        },
        Err(Err::Failure(error)) => LegacyOutcome::Failure {
            reached: to_offset(error.remaining_input.cursor),
        },
        Err(Err::Incomplete(_)) => {
            panic!("legacy module-import production requested more input for {input:?}")
        }
    }
}

fn canonical_snapshot(input: &str, rule: RuleId) -> CanonicalSourceRuleSnapshot {
    let source = TextSnapshot::new(DocumentId(0x2e), Revision(0), input)
        .expect("short direct parity probe must form a source snapshot");
    parse_canonical_phase_2e_rule_for_test(source, rule, ParseConfig::default())
        .expect("every direct parity rule must be supported")
}

fn assert_matched(
    contract: Contract,
    input: &str,
    legacy: LegacyMatch,
    canonical: CanonicalSourceRuleSnapshot,
) {
    assert_eq!(
        canonical.outcome,
        CanonicalRuleOutcome::Matched,
        "{} should cleanly match {input:?}; canonical diagnostics: {:?}",
        contract.name,
        canonical.diagnostics,
    );
    assert!(
        canonical.is_strictly_clean(),
        "{} should have a clean canonical result for {input:?}: {:?}",
        contract.name,
        canonical.diagnostics,
    );
    assert_eq!(
        canonical.consumed.start,
        TextSize::ZERO,
        "{} should begin at the probe start for {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.consumed.end, legacy.prefix.consumed,
        "{} consumed a different prefix for {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.source.byte_len() - canonical.consumed.end,
        legacy.prefix.remaining,
        "{} left a different suffix for {input:?}",
        contract.name,
    );
    validate_lossless_range(&canonical.root, &canonical.source, canonical.consumed).unwrap_or_else(
        |error| {
            panic!(
                "{} produced a non-lossless fragment for {input:?}: {error:?}",
                contract.name,
            )
        },
    );
    assert_eq!(
        reconstruct_source_range(&canonical.root, &canonical.source, canonical.consumed)
            .expect("validated canonical fragment must reconstruct"),
        input[..legacy.prefix.consumed.to_usize()],
        "{} reconstructed a different accepted prefix for {input:?}",
        contract.name,
    );

    match contract.kind {
        Some(kind) => {
            let node = find_node(&canonical.syntax(), kind).unwrap_or_else(|| {
                panic!(
                    "{} should emit {kind:?} for accepted probe {input:?}",
                    contract.name,
                )
            });
            let lowered = lower_phase_2e_module_import_value(&node).unwrap_or_else(|diagnostics| {
                panic!(
                    "{} should lower {kind:?} for {input:?}: {diagnostics:?}",
                    contract.name,
                )
            });
            assert_eq!(
                ParityValue::Node(lowered),
                legacy.value,
                "{} lowered a different legacy value for {input:?}",
                contract.name,
            );
        }
        None => assert_eq!(
            legacy.value,
            ParityValue::Transparent,
            "{} must remain a transparent acceptance rule for {input:?}",
            contract.name,
        ),
    }
}

fn assert_error(
    contract: Contract,
    input: &str,
    reached: TextSize,
    canonical: CanonicalSourceRuleSnapshot,
) {
    assert_eq!(
        canonical.outcome,
        CanonicalRuleOutcome::NoMatch,
        "{} should remain noncommitting for rejected probe {input:?}",
        contract.name,
    );
    assert!(
        !canonical.matched,
        "{} unexpectedly matched {input:?}",
        contract.name
    );
    assert!(
        canonical.diagnostics.is_empty(),
        "{} emitted diagnostics for a noncommitting probe {input:?}: {:?}",
        contract.name,
        canonical.diagnostics,
    );
    assert_eq!(
        canonical.consumed,
        TextRange::empty(TextSize::ZERO),
        "{} retained source after noncommitting probe {input:?}",
        contract.name,
    );
    assert!(
        reached <= TextSize::from_u32(input.len() as u32),
        "legacy {} failure point exceeded the probe for {input:?}",
        contract.name,
    );
}

fn assert_failure(
    contract: Contract,
    input: &str,
    reached: TextSize,
    canonical: CanonicalSourceRuleSnapshot,
) {
    assert_eq!(
        canonical.outcome,
        CanonicalRuleOutcome::Committed,
        "{} should retain its local commitment for {input:?}",
        contract.name,
    );
    assert!(
        canonical.matched,
        "{} should retain {input:?}",
        contract.name
    );
    assert!(
        !canonical.diagnostics.is_empty(),
        "{} should report its retained malformed prefix for {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.consumed.start,
        TextSize::ZERO,
        "{} should retain from the probe start for {input:?}",
        contract.name,
    );
    assert_eq!(
        canonical.consumed.end, reached,
        "{} retained a different malformed prefix for {input:?}",
        contract.name,
    );
    validate_lossless_range(&canonical.root, &canonical.source, canonical.consumed).unwrap_or_else(
        |error| {
            panic!(
                "{} produced a non-lossless retained fragment for {input:?}: {error:?}",
                contract.name,
            )
        },
    );
    assert_eq!(
        reconstruct_source_range(&canonical.root, &canonical.source, canonical.consumed)
            .expect("validated canonical fragment must reconstruct"),
        input[..reached.to_usize()],
        "{} reconstructed a different retained prefix for {input:?}",
        contract.name,
    );
    let primary = canonical
        .diagnostics
        .iter()
        .next()
        .and_then(|diagnostic| {
            diagnostic
                .primary
                .resolve(canonical.source.revision(), &canonical.nodes)
        })
        .unwrap_or_else(|| {
            panic!(
                "{} should anchor its first diagnostic for {input:?}",
                contract.name,
            )
        });
    assert_eq!(
        primary.start, reached,
        "{} diagnostic should begin at the legacy failure point for {input:?}",
        contract.name,
    );
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn assert_direct_parity_probe(contract: Contract, input: &str) {
    let legacy = legacy_outcome(input, contract.parser);
    let canonical = canonical_snapshot(input, contract.rule);
    match legacy {
        LegacyOutcome::Matched(legacy) => assert_matched(contract, input, legacy, canonical),
        LegacyOutcome::Error { reached } => assert_error(contract, input, reached, canonical),
        LegacyOutcome::Failure { reached } => assert_failure(contract, input, reached, canonical),
    }
}

#[test]
fn phase_2e_direct_module_import_rules_match_legacy_across_ninety_five_probes() {
    let contracts = phase_2e_contracts();
    assert_eq!(contracts.len(), 19);
    assert_eq!(PHASE_2E_IMPORT_RULES.len(), contracts.len());
    for rule in PHASE_2E_IMPORT_RULES {
        assert_eq!(
            contracts
                .iter()
                .filter(|contract| contract.rule == *rule)
                .count(),
            1,
            "each closed Phase 2E import rule needs exactly one parity contract: {rule}",
        );
    }

    let mut probe_count = 0;
    for contract in contracts {
        for input in contract.probes {
            probe_count += 1;
            assert_direct_parity_probe(contract, input);
        }
    }
    assert_eq!(probe_count, 95);
}

#[test]
fn phase_2e_boundary_regressions_match_legacy_failure_locations() {
    let contracts = phase_2e_contracts();
    let cases = [
        (rules::IMPORT_GROUP_ITEMS, "sin,"),
        (rules::MODULE_IMPORT, "+> math/{sin,"),
        (rules::MODULE_IMPORT_PATH, "math/_/x"),
        (rules::MODULE_IMPORT, "+> math/_/x"),
        (rules::IMPORT_GROUP_ITEMS, "math/_,cos"),
        (rules::MODULE_IMPORT, "+> math/{math/_,cos}"),
    ];
    assert_eq!(cases.len(), 6);

    for (rule, input) in cases {
        let contract = contracts
            .iter()
            .find(|contract| contract.rule == rule)
            .copied()
            .unwrap_or_else(|| panic!("missing direct parity contract for {rule}"));
        assert_direct_parity_probe(contract, input);
    }
}
