//! Canonical module-import productions for the Phase 2E closed island.
//!
//! This module deliberately stops at module imports. Source-import
//! declarations and the complete code-level alternative remain outside this
//! phase, so a direct `module-import` parse retains only its own prefix
//! behavior.

use alloc::string::String;

use crate::document::{ExpectedSyntax, RuleId, SyntaxKind};

use super::super::Parser;
use super::super::recovery::{self, RecoveryClass};
use super::super::rule::rules;
use super::base;
use super::combinator::{self, Attempt};

/// The complete closed module-import set directly ported by Phase 2E.
pub(crate) const PHASE_2E_IMPORT_RULES: &[RuleId; 19] = &[
    rules::MODULE_IMPORT_NAME_SEGMENT,
    rules::MODULE_IMPORT_INTRINSIC_SEGMENT,
    rules::MODULE_IMPORT_PATH_SEGMENT,
    rules::MODULE_IMPORT_PATH,
    rules::MODULE_IMPORT_ALIAS_SEGMENT,
    rules::MODULE_IMPORT_ALIAS_PATH,
    rules::MODULE_IMPORT_VALUE_ALIAS,
    rules::CONTEXT_IMPORT_ALIAS_SEGMENT,
    rules::MODULE_IMPORT_CONTEXT_ALIAS,
    rules::MODULE_IMPORT_ALIAS,
    rules::MODULE_ROOT,
    rules::IMPORT_ALIAS_OPERATOR,
    rules::IMPORT_GROUP_SEPARATOR,
    rules::IMPORT_GROUP_ITEM,
    rules::IMPORT_GROUP_ITEMS,
    rules::ALIASED_ITEM_IMPORT,
    rules::MODULE_SUFFIX_IMPORT,
    rules::MODULE_ONLY_IMPORT,
    rules::MODULE_IMPORT,
];

/// Whether `rule` belongs to the Phase 2E closed module-import layer.
pub(crate) fn supports(rule: RuleId) -> bool {
    PHASE_2E_IMPORT_RULES.contains(&rule)
}

/// Dispatch one exact Phase 2E module-import production.
pub(crate) fn parse_rule(parser: &mut Parser<'_>, rule: RuleId) -> Option<Attempt> {
    supports(rule).then(|| match rule {
        rules::MODULE_IMPORT_NAME_SEGMENT => parse_module_import_name_segment(parser),
        rules::MODULE_IMPORT_INTRINSIC_SEGMENT => parse_module_import_intrinsic_segment(parser),
        rules::MODULE_IMPORT_PATH_SEGMENT => parse_module_import_path_segment(parser),
        rules::MODULE_IMPORT_PATH => parse_module_import_path(parser),
        rules::MODULE_IMPORT_ALIAS_SEGMENT => parse_module_import_alias_segment(parser),
        rules::MODULE_IMPORT_ALIAS_PATH => parse_module_import_alias_path(parser),
        rules::MODULE_IMPORT_VALUE_ALIAS => parse_module_import_value_alias(parser),
        rules::CONTEXT_IMPORT_ALIAS_SEGMENT => parse_context_import_alias_segment(parser),
        rules::MODULE_IMPORT_CONTEXT_ALIAS => parse_module_import_context_alias(parser),
        rules::MODULE_IMPORT_ALIAS => parse_module_import_alias(parser),
        rules::MODULE_ROOT => parse_module_root(parser),
        rules::IMPORT_ALIAS_OPERATOR => parse_import_alias_operator(parser),
        rules::IMPORT_GROUP_SEPARATOR => parse_import_group_separator(parser),
        rules::IMPORT_GROUP_ITEM => parse_import_group_item(parser),
        rules::IMPORT_GROUP_ITEMS => parse_import_group_items(parser),
        rules::ALIASED_ITEM_IMPORT => parse_aliased_item_import(parser),
        rules::MODULE_SUFFIX_IMPORT => parse_module_suffix_import(parser),
        rules::MODULE_ONLY_IMPORT => parse_module_only_import(parser),
        rules::MODULE_IMPORT => parse_module_import(parser),
        _ => unreachable!("Phase 2E support guard rejects every other RuleId"),
    })
}

/// Parse a named module-import path segment.
pub(crate) fn parse_module_import_name_segment(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_IMPORT_NAME_SEGMENT, |parser| {
        let segment = parser.start();
        if !base::parse_rule(parser, rules::IDENTIFIER_PATH_SEGMENT) {
            segment.abandon(parser);
            return Attempt::NoMatch;
        }
        segment.complete(parser, SyntaxKind::ModuleImportNameSegment);
        Attempt::Matched
    })
}

/// Parse an intrinsic `_name` path segment.
///
/// The underscore is a local commitment: once present, the required name is
/// recovered structurally instead of allowing a surrounding path alternative
/// to reinterpret the prefix.
pub(crate) fn parse_module_import_intrinsic_segment(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_IMPORT_INTRINSIC_SEGMENT, |parser| {
        let segment = parser.start();
        if !base::parse_rule(parser, rules::UNDERSCORE) {
            segment.abandon(parser);
            return Attempt::NoMatch;
        }
        if parse_module_import_name_segment(parser) == Attempt::NoMatch {
            insert_missing_production(
                parser,
                "syntax/missing-module-import-intrinsic-name",
                "expected a module-import name after `_`",
                "module-import-name-segment",
            );
            segment.complete(parser, SyntaxKind::ModuleImportIntrinsicSegment);
            return Attempt::Committed;
        }
        segment.complete(parser, SyntaxKind::ModuleImportIntrinsicSegment);
        Attempt::Matched
    })
}

/// Parse the ordered intrinsic-or-name segment choice.
pub(crate) fn parse_module_import_path_segment(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_IMPORT_PATH_SEGMENT, |parser| {
        let segment = parser.start();
        let result = first_accepted(
            parser,
            &[
                parse_module_import_intrinsic_segment,
                parse_module_import_name_segment,
            ],
        );
        if result == Attempt::NoMatch {
            segment.abandon(parser);
            return Attempt::NoMatch;
        }
        segment.complete(parser, SyntaxKind::ModuleImportPathSegment);
        result
    })
}

/// Parse a slash-separated module-import path.
pub(crate) fn parse_module_import_path(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_IMPORT_PATH, |parser| {
        let path = parser.start();
        let result = parse_module_import_path_segment(parser);
        if result == Attempt::NoMatch {
            path.abandon(parser);
            return Attempt::NoMatch;
        }
        if result == Attempt::Committed {
            path.complete(parser, SyntaxKind::ModuleImportPath);
            return Attempt::Committed;
        }

        loop {
            let checkpoint = parser.checkpoint();
            if !base::parse_rule(parser, rules::SLASH) {
                parser.rewind(checkpoint);
                break;
            }

            match parse_module_import_path_segment(parser) {
                Attempt::NoMatch => {
                    // A repeated pair owns its slash only when its segment
                    // also succeeds. This leaves a trailing slash for the
                    // enclosing production's own recovery point.
                    parser.rewind(checkpoint);
                    break;
                }
                Attempt::Matched => {}
                Attempt::Committed => {
                    path.complete(parser, SyntaxKind::ModuleImportPath);
                    return Attempt::Committed;
                }
            }
            if parser.is_halted() {
                break;
            }
        }

        path.complete(parser, SyntaxKind::ModuleImportPath);
        Attempt::Matched
    })
}

/// Parse one value-alias path segment.
pub(crate) fn parse_module_import_alias_segment(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_IMPORT_ALIAS_SEGMENT, |parser| {
        let segment = parser.start();
        if !base::parse_rule(parser, rules::IDENTIFIER_PATH_SEGMENT) {
            segment.abandon(parser);
            return Attempt::NoMatch;
        }
        segment.complete(parser, SyntaxKind::ModuleImportAliasSegment);
        Attempt::Matched
    })
}

/// Parse a slash-separated value-alias path.
pub(crate) fn parse_module_import_alias_path(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_IMPORT_ALIAS_PATH, |parser| {
        let path = parser.start();
        let mut result = parse_module_import_alias_segment(parser);
        if result == Attempt::NoMatch {
            path.abandon(parser);
            return Attempt::NoMatch;
        }

        loop {
            let checkpoint = parser.checkpoint();
            if !base::parse_rule(parser, rules::SLASH) {
                parser.rewind(checkpoint);
                break;
            }
            match parse_module_import_alias_segment(parser) {
                Attempt::NoMatch => {
                    parser.rewind(checkpoint);
                    break;
                }
                Attempt::Matched => {}
                Attempt::Committed => result = Attempt::Committed,
            }
            if parser.is_halted() {
                break;
            }
        }

        path.complete(parser, SyntaxKind::ModuleImportAliasPath);
        result
    })
}

/// Parse a value-alias wrapper.
pub(crate) fn parse_module_import_value_alias(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_IMPORT_VALUE_ALIAS, |parser| {
        let alias = parser.start();
        let result = parse_module_import_alias_path(parser);
        if result == Attempt::NoMatch {
            alias.abandon(parser);
            return Attempt::NoMatch;
        }
        alias.complete(parser, SyntaxKind::ModuleImportValueAlias);
        result
    })
}

/// Parse the identifier portion of an `@context` alias.
pub(crate) fn parse_context_import_alias_segment(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::CONTEXT_IMPORT_ALIAS_SEGMENT, |parser| {
        let segment = parser.start();
        if !base::parse_rule(parser, rules::ALPHA_TOKEN) {
            segment.abandon(parser);
            return Attempt::NoMatch;
        }

        loop {
            let before = parser.offset();
            if !base::parse_rule(parser, rules::ALPHA_TOKEN)
                && !base::parse_rule(parser, rules::DIGIT_TOKEN)
                && !base::parse_rule(parser, rules::DASH)
            {
                break;
            }
            if parser.offset() == before || parser.is_halted() {
                break;
            }
        }

        segment.complete(parser, SyntaxKind::ContextImportAliasSegment);
        Attempt::Matched
    })
}

/// Parse a context alias and reject slash-continuation without consuming it.
///
/// This direct grammar rule remains noncommitting. The enclosing
/// `module-import` owns the special top-level `@` recovery point.
pub(crate) fn parse_module_import_context_alias(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_IMPORT_CONTEXT_ALIAS, |parser| {
        let alias = parser.start();
        if !base::parse_rule(parser, rules::AT) {
            alias.abandon(parser);
            return Attempt::NoMatch;
        }
        if parse_context_import_alias_segment(parser) == Attempt::NoMatch {
            alias.abandon(parser);
            return Attempt::NoMatch;
        }
        if starts_rule(parser, rules::SLASH) {
            alias.abandon(parser);
            return Attempt::NoMatch;
        }
        alias.complete(parser, SyntaxKind::ModuleImportContextAlias);
        Attempt::Matched
    })
}

/// Parse the ordered context-or-value alias choice.
pub(crate) fn parse_module_import_alias(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_IMPORT_ALIAS, |parser| {
        let alias = parser.start();
        let result = first_accepted(
            parser,
            &[
                parse_module_import_context_alias,
                parse_module_import_value_alias,
            ],
        );
        if result == Attempt::NoMatch {
            alias.abandon(parser);
            return Attempt::NoMatch;
        }
        alias.complete(parser, SyntaxKind::ModuleImportAlias);
        result
    })
}

/// Parse the imported module root.
pub(crate) fn parse_module_root(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_ROOT, |parser| {
        let root = parser.start();
        if !base::parse_rule(parser, rules::IDENTIFIER_PATH_SEGMENT) {
            root.abandon(parser);
            return Attempt::NoMatch;
        }
        root.complete(parser, SyntaxKind::ModuleRoot);
        Attempt::Matched
    })
}

/// Parse the transparent `:=` import alias operator with surrounding
/// horizontal space only.
pub(crate) fn parse_import_alias_operator(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::IMPORT_ALIAS_OPERATOR, |parser| {
        if !base::parse_rule(parser, rules::SPACE_TAB0)
            || !base::parse_rule(parser, rules::COLON)
            || !base::parse_rule(parser, rules::EQUAL)
            || !base::parse_rule(parser, rules::SPACE_TAB0)
        {
            return Attempt::NoMatch;
        }
        Attempt::Matched
    })
}

/// Parse a transparent item-group separator.
pub(crate) fn parse_import_group_separator(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::IMPORT_GROUP_SEPARATOR, |parser| {
        if base::parse_rule(parser, rules::LIST_SEPARATOR)
            || base::parse_rule(parser, rules::WHITESPACE1)
        {
            Attempt::Matched
        } else {
            Attempt::NoMatch
        }
    })
}

/// Parse one module-import group item.
pub(crate) fn parse_import_group_item(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::IMPORT_GROUP_ITEM, |parser| {
        let item = parser.start();
        let result = parse_module_import_path(parser);
        if result == Attempt::NoMatch {
            item.abandon(parser);
            return Attempt::NoMatch;
        }
        item.complete(parser, SyntaxKind::ImportGroupItem);
        result
    })
}

/// Parse nonempty import-group items. Repeated separator/item pairs are
/// transactional so trailing whitespace remains available to the final
/// `whitespace0` production.
pub(crate) fn parse_import_group_items(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::IMPORT_GROUP_ITEMS, |parser| {
        let items = parser.start();
        let _ = base::parse_rule(parser, rules::WHITESPACE0);
        let result = parse_import_group_item(parser);
        if result == Attempt::NoMatch {
            items.abandon(parser);
            return Attempt::NoMatch;
        }
        if result == Attempt::Committed {
            items.complete(parser, SyntaxKind::ImportGroupItems);
            return Attempt::Committed;
        }

        loop {
            let checkpoint = parser.checkpoint();
            let separator = parse_import_group_separator(parser);
            if separator == Attempt::NoMatch {
                parser.rewind(checkpoint);
                break;
            }
            match parse_import_group_item(parser) {
                Attempt::NoMatch => {
                    // The repeated separator/item pair is transactional. An
                    // ordinary failed item leaves both pieces for the
                    // enclosing braced group to recover, exactly like the
                    // legacy `many0(preceded(..))` production.
                    parser.rewind(checkpoint);
                    break;
                }
                Attempt::Matched => {}
                Attempt::Committed => {
                    items.complete(parser, SyntaxKind::ImportGroupItems);
                    return Attempt::Committed;
                }
            }
            if parser.is_halted() {
                break;
            }
        }

        let _ = base::parse_rule(parser, rules::WHITESPACE0);
        items.complete(parser, SyntaxKind::ImportGroupItems);
        Attempt::Matched
    })
}

/// Parse `alias := module/item`, committing only after the complete alias
/// operator has matched. A plain identifier remains available to the module
/// only-import alternative.
pub(crate) fn parse_aliased_item_import(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::ALIASED_ITEM_IMPORT, |parser| {
        let import = parser.start();
        let alias = parse_module_import_alias(parser);
        if alias == Attempt::NoMatch {
            import.abandon(parser);
            return Attempt::NoMatch;
        }

        if parse_import_alias_operator(parser) == Attempt::NoMatch {
            import.abandon(parser);
            return Attempt::NoMatch;
        }

        let module = parse_module_root(parser);
        if module == Attempt::NoMatch {
            insert_missing_production(
                parser,
                "syntax/missing-module-import-alias-target",
                "expected a module root after the import alias operator",
                "module-root",
            );
            import.complete(parser, SyntaxKind::AliasedItemImport);
            return Attempt::Committed;
        }
        if !base::parse_rule(parser, rules::SLASH) {
            insert_missing_token(
                parser,
                "syntax/missing-module-import-aliased-item-separator",
                "expected `/` between the imported module and item",
                SyntaxKind::Slash,
                "/",
            );
            import.complete(parser, SyntaxKind::AliasedItemImport);
            return Attempt::Committed;
        }
        let item = parse_module_import_path(parser);
        if item == Attempt::NoMatch {
            insert_missing_production(
                parser,
                "syntax/missing-module-import-aliased-item",
                "expected an imported item path after `/`",
                "module-import-path",
            );
            import.complete(parser, SyntaxKind::AliasedItemImport);
            return Attempt::Committed;
        }

        import.complete(parser, SyntaxKind::AliasedItemImport);
        if module == Attempt::Committed || item == Attempt::Committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    })
}

/// Parse module-root slash suffix imports. The slash is the local commitment
/// point; a missing suffix is recovered without falling back to a module-only
/// import.
pub(crate) fn parse_module_suffix_import(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_SUFFIX_IMPORT, |parser| {
        let import = parser.start();
        let module = parse_module_root(parser);
        if module == Attempt::NoMatch || !base::parse_rule(parser, rules::SLASH) {
            import.abandon(parser);
            return Attempt::NoMatch;
        }

        let result = if base::parse_rule(parser, rules::ASTERISK) {
            Attempt::Matched
        } else if base::parse_rule(parser, rules::LEFT_BRACE) {
            parse_braced_group_suffix(parser)
        } else {
            match parse_module_import_path(parser) {
                Attempt::NoMatch => {
                    insert_missing_production(
                        parser,
                        "syntax/missing-module-import-suffix",
                        "expected a module import suffix after `/`",
                        "module-import-path",
                    );
                    Attempt::Committed
                }
                result => result,
            }
        };

        import.complete(parser, SyntaxKind::ModuleSuffixImport);
        if module == Attempt::Committed || result == Attempt::Committed {
            Attempt::Committed
        } else {
            Attempt::Matched
        }
    })
}

/// Parse a braced import group after its opening brace has committed.
fn parse_braced_group_suffix(parser: &mut Parser<'_>) -> Attempt {
    let mut result = parse_import_group_items(parser);
    if result == Attempt::NoMatch {
        insert_missing_production(
            parser,
            "syntax/missing-module-import-group-item",
            "expected an item in the module import group",
            "import-group-item",
        );
        result = Attempt::Committed;
    } else if result == Attempt::Matched && starts_rule(parser, rules::LIST_SEPARATOR) {
        // `import-group-items` left the incomplete repeated separator/item
        // pair untouched. The braced parent owns recovery at this boundary.
        insert_missing_production(
            parser,
            "syntax/missing-module-import-group-item",
            "expected an item after the module import group separator",
            "import-group-item",
        );
        result = Attempt::Committed;
    }

    if !base::parse_rule(parser, rules::RIGHT_BRACE) {
        insert_missing_token(
            parser,
            "syntax/unclosed-module-import-group",
            "expected `}` to close the module import group",
            SyntaxKind::RightBrace,
            "}",
        );
        result = Attempt::Committed;
    }
    result
}

/// Parse a module-only import and reject a following slash without consuming
/// it so the suffix production remains its owner.
pub(crate) fn parse_module_only_import(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_ONLY_IMPORT, |parser| {
        let import = parser.start();
        let result = parse_module_root(parser);
        if result == Attempt::NoMatch || starts_rule(parser, rules::SLASH) {
            import.abandon(parser);
            return Attempt::NoMatch;
        }
        import.complete(parser, SyntaxKind::ModuleOnlyImport);
        result
    })
}

/// Parse one complete module import. The shared `+>` prefix remains
/// noncommitting; only a distinctive alias or suffix prefix commits locally.
pub(crate) fn parse_module_import(parser: &mut Parser<'_>) -> Attempt {
    combinator::transactional(parser, rules::MODULE_IMPORT, |parser| {
        let import = parser.start();
        let _ = base::parse_rule(parser, rules::WHITESPACE0);
        if !base::parse_rule(parser, rules::IMPORT_SIGIL) {
            import.abandon(parser);
            return Attempt::NoMatch;
        }
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);

        let result = if starts_rule(parser, rules::AT) {
            match parse_aliased_item_import(parser) {
                Attempt::NoMatch => recover_context_alias_prefix(parser),
                result => result,
            }
        } else {
            first_accepted(
                parser,
                &[
                    parse_aliased_item_import,
                    parse_module_suffix_import,
                    parse_module_only_import,
                ],
            )
        };

        if result == Attempt::NoMatch {
            import.abandon(parser);
            return Attempt::NoMatch;
        }
        import.complete(parser, SyntaxKind::ModuleImport);
        result
    })
}

/// Keep an immediately-prefixed malformed context alias local to the
/// top-level aliased form. This is intentionally not used for a value alias:
/// plain identifiers still need the module-only fallback.
fn recover_context_alias_prefix(parser: &mut Parser<'_>) -> Attempt {
    let import = parser.start();
    let alias = parser.start();
    let context = parser.start();
    debug_assert!(base::parse_rule(parser, rules::AT));
    let segment = parse_context_import_alias_segment(parser);
    if segment == Attempt::NoMatch {
        insert_missing_production(
            parser,
            "syntax/missing-module-import-context-alias",
            "expected a context import alias after `@`",
            "context-import-alias-segment",
        );
        context.complete(parser, SyntaxKind::ModuleImportContextAlias);
        alias.complete(parser, SyntaxKind::ModuleImportAlias);
        import.complete(parser, SyntaxKind::AliasedItemImport);
        return Attempt::Committed;
    }

    if starts_rule(parser, rules::SLASH) {
        let _ = recovery::skip_error(
            parser,
            RecoveryClass::MechItem,
            "syntax/invalid-module-import-context-alias",
            "context import aliases cannot continue with `/`",
        );
        context.complete(parser, SyntaxKind::ModuleImportContextAlias);
        alias.complete(parser, SyntaxKind::ModuleImportAlias);
        import.complete(parser, SyntaxKind::AliasedItemImport);
        return Attempt::Committed;
    }

    context.complete(parser, SyntaxKind::ModuleImportContextAlias);
    alias.complete(parser, SyntaxKind::ModuleImportAlias);

    if parse_import_alias_operator(parser) == Attempt::NoMatch {
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
        if !base::parse_rule(parser, rules::COLON) {
            insert_missing_production(
                parser,
                "syntax/missing-module-import-alias-operator",
                "expected `:=` after the context import alias",
                "import-alias-operator",
            );
            import.complete(parser, SyntaxKind::AliasedItemImport);
            return Attempt::Committed;
        }
        if !base::parse_rule(parser, rules::EQUAL) {
            insert_missing_token(
                parser,
                "syntax/missing-module-import-alias-equal",
                "expected `=` after `:` in the import alias operator",
                SyntaxKind::Equal,
                "=",
            );
            import.complete(parser, SyntaxKind::AliasedItemImport);
            return Attempt::Committed;
        }
        let _ = base::parse_rule(parser, rules::SPACE_TAB0);
    }

    let module = parse_module_root(parser);
    if module == Attempt::NoMatch {
        insert_missing_production(
            parser,
            "syntax/missing-module-import-alias-target",
            "expected a module root after the import alias operator",
            "module-root",
        );
        import.complete(parser, SyntaxKind::AliasedItemImport);
        return Attempt::Committed;
    }
    if !base::parse_rule(parser, rules::SLASH) {
        insert_missing_token(
            parser,
            "syntax/missing-module-import-aliased-item-separator",
            "expected `/` between the imported module and item",
            SyntaxKind::Slash,
            "/",
        );
        import.complete(parser, SyntaxKind::AliasedItemImport);
        return Attempt::Committed;
    }
    let item = parse_module_import_path(parser);
    if item == Attempt::NoMatch {
        insert_missing_production(
            parser,
            "syntax/missing-module-import-aliased-item",
            "expected an imported item path after `/`",
            "module-import-path",
        );
        import.complete(parser, SyntaxKind::AliasedItemImport);
        return Attempt::Committed;
    }

    import.complete(parser, SyntaxKind::AliasedItemImport);
    Attempt::Committed
}

fn first_accepted(
    parser: &mut Parser<'_>,
    alternatives: &[fn(&mut Parser<'_>) -> Attempt],
) -> Attempt {
    for alternative in alternatives {
        let result = alternative(parser);
        if result != Attempt::NoMatch {
            return result;
        }
        if parser.is_halted() {
            return Attempt::NoMatch;
        }
    }
    Attempt::NoMatch
}

fn starts_rule(parser: &mut Parser<'_>, rule: RuleId) -> bool {
    let checkpoint = parser.checkpoint();
    let matched = base::parse_rule(parser, rule);
    parser.rewind(checkpoint);
    matched
}

fn insert_missing_production(parser: &mut Parser<'_>, code: &str, message: &str, production: &str) {
    combinator::insert_missing(
        parser,
        code,
        message,
        ExpectedSyntax::Production(String::from(production)),
        None,
        None,
    );
}

fn insert_missing_token(
    parser: &mut Parser<'_>,
    code: &str,
    message: &str,
    token: SyntaxKind,
    text: &str,
) {
    combinator::insert_missing(
        parser,
        code,
        message,
        ExpectedSyntax::Token(token),
        Some(token),
        Some(text),
    );
}
