use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use mech_core::{
    Grammar as LegacyGrammar, GrammarExpression as LegacyExpression,
    GrammarIdentifier as LegacyIdentifier, Rule as LegacyRule, Token as LegacyToken, TokenKind,
};

use crate::document::ast::grammar::{
    GrammarDefinitionSyntax, GrammarDocumentSyntax, GrammarExpressionSyntax, GrammarFactorSyntax,
    GrammarGroupSyntax, GrammarIdentifierSyntax, GrammarListSyntax, GrammarNotSyntax,
    GrammarOptionalSyntax, GrammarPeekSyntax, GrammarRangeSyntax, GrammarRepeat0Syntax,
    GrammarRepeat1Syntax, GrammarRuleSyntax, GrammarSyntax, GrammarTermSyntax,
    GrammarTerminalSyntax, GrammarTerminalTokenSyntax,
};
use crate::document::{
    AstNode, Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticPhase, DiagnosticStore,
    DiagnosticTags, IdGenerator, NodeFlags, Severity, SyntaxElement, SyntaxKind, SyntaxNode,
    SyntaxSnapshot, SyntaxToken, TextRange, TokenFlags,
};

use super::source;

const IGNORED_VALUE_FLAGS: TokenFlags = TokenFlags(TokenFlags::TRIVIA.0 | TokenFlags::SYNTHETIC.0);
const INVALID_TOKEN_FLAGS: TokenFlags = TokenFlags(TokenFlags::MISSING.0 | TokenFlags::ERROR.0);
const INVALID_NODE_FLAGS: NodeFlags = NodeFlags(
    NodeFlags::ERROR.0
        | NodeFlags::MISSING.0
        | NodeFlags::CONTAINS_ERROR.0
        | NodeFlags::CONTAINS_MISSING.0,
);

#[derive(Clone, Debug)]
struct LowerFailure {
    range: TextRange,
    message: String,
}

impl LowerFailure {
    fn at_node(node: &SyntaxNode, message: impl Into<String>) -> Self {
        Self {
            range: node.range(),
            message: message.into(),
        }
    }

    fn at_token(token: &SyntaxToken, message: impl Into<String>) -> Self {
        Self {
            range: token.range(),
            message: message.into(),
        }
    }
}

type LowerResult<T> = Result<T, LowerFailure>;

pub fn lower_legacy_grammar(snapshot: &SyntaxSnapshot) -> Result<LegacyGrammar, DiagnosticStore> {
    if !snapshot.diagnostics.is_empty() {
        return Err(snapshot.diagnostics.clone());
    }

    let root = snapshot.syntax();
    if let Some(range) = first_erroneous_range(&root) {
        return Err(failure_store(
            snapshot,
            LowerFailure {
                range,
                message: String::from("grammar compatibility lowering requires error-free syntax"),
            },
        ));
    }

    let lowered = (|| {
        let document = GrammarDocumentSyntax::cast(root.clone()).ok_or_else(|| {
            LowerFailure::at_node(&root, "expected a canonical grammar document root")
        })?;
        lower_document(&document)
    })();

    lowered.map_err(|failure| failure_store(snapshot, failure))
}

fn lower_document(document: &GrammarDocumentSyntax) -> LowerResult<LegacyGrammar> {
    ensure_only_child_kinds(document.syntax(), &[SyntaxKind::Grammar])?;
    ensure_direct_tokens(document.syntax(), &[])?;
    let grammar = require_one(
        grammar_children(document.syntax()),
        document.syntax(),
        "grammar",
    )?;
    lower_grammar(&grammar)
}

fn lower_grammar(grammar: &GrammarSyntax) -> LowerResult<LegacyGrammar> {
    ensure_only_child_kinds(grammar.syntax(), &[SyntaxKind::GrammarRule])?;
    ensure_direct_tokens(grammar.syntax(), &[])?;
    let rules = grammar.rules();
    if rules.is_empty() {
        return Err(LowerFailure::at_node(
            grammar.syntax(),
            "canonical grammar must contain at least one grammar rule",
        ));
    }
    let rules = rules
        .iter()
        .map(lower_rule)
        .collect::<LowerResult<Vec<_>>>()?;
    Ok(LegacyGrammar { rules })
}

fn lower_rule(rule: &GrammarRuleSyntax) -> LowerResult<LegacyRule> {
    ensure_only_child_kinds(
        rule.syntax(),
        &[SyntaxKind::GrammarIdentifier, SyntaxKind::GrammarExpression],
    )?;
    ensure_rule_tokens(rule.syntax())?;
    let name = require_one(
        identifier_children(rule.syntax()),
        rule.syntax(),
        "grammar-rule name",
    )?;
    let expression = require_one(
        expression_children(rule.syntax()),
        rule.syntax(),
        "grammar-rule expression",
    )?;
    Ok(LegacyRule {
        name: lower_identifier(&name)?,
        expr: lower_expression(&expression)?,
    })
}

fn lower_identifier(identifier: &GrammarIdentifierSyntax) -> LowerResult<LegacyIdentifier> {
    ensure_only_child_kinds(identifier.syntax(), &[])?;
    let name = lower_value_token(
        identifier.syntax(),
        &identifier.name_tokens(),
        TokenKind::Alpha,
        "grammar identifier",
    )?;
    Ok(LegacyIdentifier { name })
}

fn lower_expression(expression: &GrammarExpressionSyntax) -> LowerResult<LegacyExpression> {
    ensure_only_child_kinds(expression.syntax(), &[SyntaxKind::GrammarTerm])?;
    let terms = expression.terms();
    if terms.is_empty() {
        return Err(LowerFailure::at_node(
            expression.syntax(),
            "grammar expression is missing its required term",
        ));
    }
    let separators = vec![SyntaxKind::Bar; terms.len().saturating_sub(1)];
    ensure_direct_tokens(expression.syntax(), &separators)?;
    let mut lowered = terms
        .iter()
        .map(lower_term)
        .collect::<LowerResult<Vec<_>>>()?;
    if lowered.len() == 1 {
        Ok(lowered.remove(0))
    } else {
        Ok(LegacyExpression::Choice(lowered))
    }
}

fn lower_term(term: &GrammarTermSyntax) -> LowerResult<LegacyExpression> {
    ensure_only_child_kinds(term.syntax(), &[SyntaxKind::GrammarFactor])?;
    let factors = term.factors();
    if factors.is_empty() {
        return Err(LowerFailure::at_node(
            term.syntax(),
            "grammar term is missing its required factor",
        ));
    }
    let separators = vec![SyntaxKind::Comma; factors.len().saturating_sub(1)];
    ensure_direct_tokens(term.syntax(), &separators)?;
    let mut lowered = factors
        .iter()
        .map(lower_factor)
        .collect::<LowerResult<Vec<_>>>()?;
    if lowered.len() == 1 {
        Ok(lowered.remove(0))
    } else {
        Ok(LegacyExpression::Sequence(lowered))
    }
}

fn lower_factor(factor: &GrammarFactorSyntax) -> LowerResult<LegacyExpression> {
    const FACTOR_KINDS: &[SyntaxKind] = &[
        SyntaxKind::GrammarRepeat0,
        SyntaxKind::GrammarRepeat1,
        SyntaxKind::GrammarOptional,
        SyntaxKind::GrammarPeek,
        SyntaxKind::GrammarNot,
        SyntaxKind::GrammarGroup,
        SyntaxKind::GrammarList,
        SyntaxKind::GrammarDefinition,
        SyntaxKind::GrammarRange,
        SyntaxKind::GrammarTerminal,
    ];
    ensure_only_child_kinds(factor.syntax(), FACTOR_KINDS)?;
    ensure_direct_tokens(factor.syntax(), &[])?;
    if factor.syntax().children().count() != 1 {
        return Err(LowerFailure::at_node(
            factor.syntax(),
            "grammar factor must contain exactly one factor production",
        ));
    }

    if let Some(value) = factor.repeat0() {
        lower_repeat0(&value)
    } else if let Some(value) = factor.repeat1() {
        lower_repeat1(&value)
    } else if let Some(value) = factor.optional() {
        lower_optional(&value)
    } else if let Some(value) = factor.peek() {
        lower_peek(&value)
    } else if let Some(value) = factor.not() {
        lower_not(&value)
    } else if let Some(value) = factor.group() {
        lower_group(&value)
    } else if let Some(value) = factor.list() {
        lower_list(&value)
    } else if let Some(value) = factor.definition() {
        lower_definition(&value)
    } else if let Some(value) = factor.range() {
        lower_range(&value)
    } else if let Some(value) = factor.terminal() {
        lower_terminal(&value)
    } else {
        Err(LowerFailure::at_node(
            factor.syntax(),
            "grammar factor has no recognized production",
        ))
    }
}

fn lower_definition(definition: &GrammarDefinitionSyntax) -> LowerResult<LegacyExpression> {
    ensure_only_child_kinds(definition.syntax(), &[SyntaxKind::GrammarIdentifier])?;
    ensure_direct_tokens(definition.syntax(), &[])?;
    let identifier = require_one(
        identifier_children(definition.syntax()),
        definition.syntax(),
        "grammar definition identifier",
    )?;
    Ok(LegacyExpression::Definition(lower_identifier(&identifier)?))
}

fn lower_repeat0(repeat: &GrammarRepeat0Syntax) -> LowerResult<LegacyExpression> {
    let factor = lower_unary_factor(
        repeat.syntax(),
        &[SyntaxKind::Asterisk],
        "zero-or-more operand",
    )?;
    Ok(LegacyExpression::Repeat0(Box::new(factor)))
}

fn lower_repeat1(repeat: &GrammarRepeat1Syntax) -> LowerResult<LegacyExpression> {
    let factor = lower_unary_factor(repeat.syntax(), &[SyntaxKind::Plus], "one-or-more operand")?;
    Ok(LegacyExpression::Repeat1(Box::new(factor)))
}

fn lower_optional(optional: &GrammarOptionalSyntax) -> LowerResult<LegacyExpression> {
    let factor = lower_unary_factor(
        optional.syntax(),
        &[SyntaxKind::Question],
        "optional operand",
    )?;
    Ok(LegacyExpression::Optional(Box::new(factor)))
}

fn lower_peek(peek: &GrammarPeekSyntax) -> LowerResult<LegacyExpression> {
    let factor = lower_unary_factor(peek.syntax(), &[SyntaxKind::RightAngle], "peek operand")?;
    Ok(LegacyExpression::Peek(Box::new(factor)))
}

fn lower_not(not: &GrammarNotSyntax) -> LowerResult<LegacyExpression> {
    let factor = lower_unary_factor(
        not.syntax(),
        &[SyntaxKind::Not],
        "negative-lookahead operand",
    )?;
    Ok(LegacyExpression::Not(Box::new(factor)))
}

fn lower_unary_factor(
    syntax: &SyntaxNode,
    tokens: &[SyntaxKind],
    name: &str,
) -> LowerResult<LegacyExpression> {
    ensure_only_child_kinds(syntax, &[SyntaxKind::GrammarFactor])?;
    ensure_direct_tokens(syntax, tokens)?;
    let factor = require_one(factor_children(syntax), syntax, name)?;
    lower_factor(&factor)
}

fn lower_list(list: &GrammarListSyntax) -> LowerResult<LegacyExpression> {
    ensure_only_child_kinds(list.syntax(), &[SyntaxKind::GrammarFactor])?;
    ensure_direct_tokens(
        list.syntax(),
        &[
            SyntaxKind::LeftBracket,
            SyntaxKind::Comma,
            SyntaxKind::RightBracket,
        ],
    )?;
    let factors = list.factors();
    if factors.len() != 2 {
        return Err(LowerFailure::at_node(
            list.syntax(),
            "grammar list must contain exactly two factors",
        ));
    }
    Ok(LegacyExpression::List(
        Box::new(lower_factor(&factors[0])?),
        Box::new(lower_factor(&factors[1])?),
    ))
}

fn lower_range(range: &GrammarRangeSyntax) -> LowerResult<LegacyExpression> {
    ensure_only_child_kinds(range.syntax(), &[SyntaxKind::GrammarTerminalToken])?;
    ensure_direct_tokens(range.syntax(), &[SyntaxKind::Period, SyntaxKind::Period])?;
    let endpoints = range.terminal_tokens();
    if endpoints.len() != 2 {
        return Err(LowerFailure::at_node(
            range.syntax(),
            "grammar range must contain exactly two terminal tokens",
        ));
    }
    Ok(LegacyExpression::Range(
        lower_terminal_token(&endpoints[0])?,
        lower_terminal_token(&endpoints[1])?,
    ))
}

fn lower_group(group: &GrammarGroupSyntax) -> LowerResult<LegacyExpression> {
    ensure_only_child_kinds(group.syntax(), &[SyntaxKind::GrammarExpression])?;
    ensure_direct_tokens(
        group.syntax(),
        &[SyntaxKind::LeftParen, SyntaxKind::RightParen],
    )?;
    let expression = require_one(
        expression_children(group.syntax()),
        group.syntax(),
        "group expression",
    )?;
    Ok(LegacyExpression::Group(Box::new(lower_expression(
        &expression,
    )?)))
}

fn lower_terminal(terminal: &GrammarTerminalSyntax) -> LowerResult<LegacyExpression> {
    ensure_only_child_kinds(terminal.syntax(), &[SyntaxKind::GrammarTerminalToken])?;
    ensure_direct_tokens(terminal.syntax(), &[])?;
    let terminal_token = require_one(
        terminal_token_children(terminal.syntax()),
        terminal.syntax(),
        "grammar terminal token",
    )?;
    Ok(LegacyExpression::Terminal(lower_terminal_token(
        &terminal_token,
    )?))
}

fn lower_terminal_token(terminal: &GrammarTerminalTokenSyntax) -> LowerResult<LegacyToken> {
    ensure_only_child_kinds(terminal.syntax(), &[])?;
    let tokens = significant_direct_tokens(terminal.syntax());
    let quote_count = tokens
        .iter()
        .filter(|token| token.kind() == SyntaxKind::Quote)
        .count();
    if quote_count != 2
        || tokens.first().map(SyntaxToken::kind) != Some(SyntaxKind::Quote)
        || tokens.last().map(SyntaxToken::kind) != Some(SyntaxKind::Quote)
    {
        return Err(LowerFailure::at_node(
            terminal.syntax(),
            "grammar terminal token requires exactly one opening and closing quote",
        ));
    }
    lower_value_token(
        terminal.syntax(),
        &terminal.content_tokens(),
        TokenKind::Any,
        "grammar terminal",
    )
}

fn lower_value_token(
    syntax: &SyntaxNode,
    tokens: &[SyntaxToken],
    kind: TokenKind,
    name: &str,
) -> LowerResult<LegacyToken> {
    let mut characters = Vec::new();
    let mut physical_start = None;
    let mut physical_end = None;

    for token in tokens {
        if token.flags().intersects(INVALID_TOKEN_FLAGS) {
            return Err(LowerFailure::at_token(
                token,
                format!("{name} contains erroneous syntax"),
            ));
        }
        if token.flags().intersects(IGNORED_VALUE_FLAGS) {
            continue;
        }
        let text = token.text().map_err(|_| {
            LowerFailure::at_token(token, format!("cannot read the physical source for {name}"))
        })?;
        let mut retained = false;
        for character in text.chars() {
            if is_legacy_grammar_whitespace(character) {
                continue;
            }
            retained = true;
            characters.push(character);
        }
        if retained {
            physical_start.get_or_insert(token.range().start);
            physical_end = Some(token.range().end);
        }
    }

    let Some(physical_start) = physical_start else {
        return Err(LowerFailure::at_node(
            syntax,
            format!("{name} has no compatibility value"),
        ));
    };
    let physical_end = physical_end.expect("a start always has an end");
    let physical = TextRange::new(physical_start, physical_end);
    let src_range = source::source_range(syntax.source(), physical).ok_or_else(|| {
        LowerFailure::at_node(
            syntax,
            format!("cannot convert the physical source range for {name}"),
        )
    })?;
    debug_assert_ne!(src_range.start.row, 0);
    debug_assert_ne!(src_range.start.col, 0);
    debug_assert_ne!(src_range.end.row, 0);
    debug_assert_ne!(src_range.end.col, 0);

    Ok(LegacyToken {
        kind,
        chars: characters,
        src_range,
    })
}

fn is_legacy_grammar_whitespace(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\r' | '\n')
}

fn first_erroneous_range(node: &SyntaxNode) -> Option<TextRange> {
    if matches!(node.kind(), SyntaxKind::Error | SyntaxKind::Missing)
        || node.flags().intersects(INVALID_NODE_FLAGS)
    {
        return Some(node.range());
    }
    for child in node.children_with_tokens() {
        match child {
            SyntaxElement::Node(child) => {
                if let Some(range) = first_erroneous_range(&child) {
                    return Some(range);
                }
            }
            SyntaxElement::Token(token) if token.flags().intersects(INVALID_TOKEN_FLAGS) => {
                return Some(token.range());
            }
            SyntaxElement::Token(_) => {}
        }
    }
    None
}

fn ensure_only_child_kinds(syntax: &SyntaxNode, allowed: &[SyntaxKind]) -> LowerResult<()> {
    if let Some(unexpected) = syntax
        .children()
        .find(|child| !allowed.contains(&child.kind()))
    {
        return Err(LowerFailure::at_node(
            &unexpected,
            format!(
                "{:?} is not a valid direct child of {:?}",
                unexpected.kind(),
                syntax.kind(),
            ),
        ));
    }
    Ok(())
}

fn ensure_direct_tokens(syntax: &SyntaxNode, expected: &[SyntaxKind]) -> LowerResult<()> {
    let found = significant_direct_tokens(syntax)
        .iter()
        .map(SyntaxToken::kind)
        .collect::<Vec<_>>();
    if found != expected {
        return Err(LowerFailure::at_node(
            syntax,
            format!(
                "{:?} requires direct tokens {expected:?}, found {found:?}",
                syntax.kind(),
            ),
        ));
    }
    Ok(())
}

fn ensure_rule_tokens(syntax: &SyntaxNode) -> LowerResult<()> {
    let found = significant_direct_tokens(syntax)
        .iter()
        .map(SyntaxToken::kind)
        .collect::<Vec<_>>();
    if matches!(
        found.as_slice(),
        [SyntaxKind::DefineOperatorToken, SyntaxKind::Semicolon]
            | [SyntaxKind::Colon, SyntaxKind::Equal, SyntaxKind::Semicolon]
    ) {
        return Ok(());
    }
    Err(LowerFailure::at_node(
        syntax,
        format!(
            "{:?} requires a physical := operator and semicolon, found {found:?}",
            syntax.kind(),
        ),
    ))
}

fn significant_direct_tokens(syntax: &SyntaxNode) -> Vec<SyntaxToken> {
    syntax
        .children_with_tokens()
        .into_iter()
        .filter_map(|element| match element {
            SyntaxElement::Token(token) if !token.flags().intersects(IGNORED_VALUE_FLAGS) => {
                Some(token)
            }
            SyntaxElement::Node(_) | SyntaxElement::Token(_) => None,
        })
        .collect()
}

fn require_one<T>(mut values: Vec<T>, syntax: &SyntaxNode, name: &str) -> LowerResult<T> {
    if values.len() != 1 {
        return Err(LowerFailure::at_node(
            syntax,
            format!("{name} must occur exactly once"),
        ));
    }
    Ok(values.remove(0))
}

macro_rules! typed_children {
    ($name:ident, $type:ty) => {
        fn $name(syntax: &SyntaxNode) -> Vec<$type> {
            syntax.children().filter_map(<$type>::cast).collect()
        }
    };
}

typed_children!(grammar_children, GrammarSyntax);
typed_children!(identifier_children, GrammarIdentifierSyntax);
typed_children!(expression_children, GrammarExpressionSyntax);
typed_children!(factor_children, GrammarFactorSyntax);
typed_children!(terminal_token_children, GrammarTerminalTokenSyntax);

fn failure_store(snapshot: &SyntaxSnapshot, failure: LowerFailure) -> DiagnosticStore {
    let mut ids = IdGenerator::new();
    let mut diagnostics = DiagnosticStore::new(snapshot.revision);
    diagnostics.push(Diagnostic {
        id: ids.diagnostic(),
        code: DiagnosticCode::from("lowering/invalid-grammar-syntax"),
        phase: DiagnosticPhase::Lowering,
        severity: Severity::Error,
        rule: None,
        context: None,
        primary: DiagnosticAnchor::Absolute {
            revision: snapshot.revision,
            range: failure.range,
        },
        labels: Vec::new(),
        expected: Vec::new(),
        found: None,
        fixes: Vec::new(),
        related: Vec::new(),
        recovery: None,
        tags: DiagnosticTags::NONE,
        message: failure.message,
    });
    diagnostics
}
