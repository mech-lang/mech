use alloc::string::String;
use alloc::vec::Vec;

use mech_core::{Paragraph, ParagraphElement, SectionElement, Token as LegacyToken, TokenKind};

use crate::document::ast::mechdown::{
    EquationSyntax, FootnoteReferenceSyntax, InlineCodeSyntax, InlineEquationSyntax,
    ParagraphTextSyntax, RawHyperlinkSyntax, ReferenceSyntax, SectionReferenceSyntax,
    ThematicBreakSyntax,
};
use crate::document::{
    AstNode, Diagnostic, DiagnosticAnchor, DiagnosticCode, DiagnosticPhase, DiagnosticStore,
    DiagnosticTags, IdGenerator, NodeFlags, Severity, SyntaxElement, SyntaxKind, SyntaxNode,
    SyntaxToken, TokenFlags,
};

use super::source;

const INVALID_NODE_FLAGS: NodeFlags = NodeFlags(
    NodeFlags::ERROR.0
        | NodeFlags::MISSING.0
        | NodeFlags::CONTAINS_ERROR.0
        | NodeFlags::CONTAINS_MISSING.0,
);
const INVALID_TOKEN_FLAGS: TokenFlags = TokenFlags(TokenFlags::MISSING.0 | TokenFlags::ERROR.0);

pub fn lower_legacy_inline_code(
    syntax: &InlineCodeSyntax,
) -> Result<ParagraphElement, DiagnosticStore> {
    lower_value(syntax.syntax(), "inline-code", |node| {
        let elements = direct_elements(node)?;
        require_delimited(
            node,
            &elements,
            SyntaxKind::Grave,
            "`",
            SyntaxKind::Grave,
            "`",
        )?;
        let mut content = lower_elements(node, &elements[1..elements.len() - 1])?;
        if content.is_empty() {
            return Ok(ParagraphElement::InlineCode(LegacyToken::default()));
        }
        let mut token = merge_tokens(&mut content, "inline-code content")?;
        token.kind = TokenKind::Text;
        Ok(ParagraphElement::InlineCode(token))
    })
}

pub fn lower_legacy_inline_equation(
    syntax: &InlineEquationSyntax,
) -> Result<ParagraphElement, DiagnosticStore> {
    lower_value(syntax.syntax(), "inline-equation", |node| {
        let elements = direct_elements(node)?;
        require_delimited(
            node,
            &elements,
            SyntaxKind::EquationSigil,
            "$$",
            SyntaxKind::EquationSigil,
            "$$",
        )?;
        let mut content = lower_elements(node, &elements[1..elements.len() - 1])?;
        let mut token = merge_tokens(&mut content, "inline-equation content")?;
        token.kind = TokenKind::Text;
        Ok(ParagraphElement::InlineEquation(token))
    })
}

pub fn lower_legacy_raw_hyperlink(
    syntax: &RawHyperlinkSyntax,
) -> Result<ParagraphElement, DiagnosticStore> {
    lower_value(syntax.syntax(), "raw-hyperlink", |node| {
        let elements = direct_elements(node)?;
        let mut tokens = lower_elements(node, &elements)?;
        let url = merge_tokens(&mut tokens, "raw-hyperlink URL")?;
        let paragraph = Paragraph {
            elements: alloc::vec![ParagraphElement::Text(url.clone())],
            error_range: None,
        };
        Ok(ParagraphElement::Hyperlink((paragraph, url)))
    })
}

pub fn lower_legacy_footnote_reference(
    syntax: &FootnoteReferenceSyntax,
) -> Result<ParagraphElement, DiagnosticStore> {
    lower_value(syntax.syntax(), "footnote-reference", |node| {
        let elements = direct_elements(node)?;
        require_delimited(
            node,
            &elements,
            SyntaxKind::FootnotePrefix,
            "[^",
            SyntaxKind::RightBracket,
            "]",
        )?;
        let mut content = lower_elements(node, &elements[1..elements.len() - 1])?;
        Ok(ParagraphElement::FootnoteReference(merge_tokens(
            &mut content,
            "footnote-reference content",
        )?))
    })
}

pub fn lower_legacy_reference(
    syntax: &ReferenceSyntax,
) -> Result<ParagraphElement, DiagnosticStore> {
    lower_value(syntax.syntax(), "reference", |node| {
        let elements = direct_elements(node)?;
        require_delimited(
            node,
            &elements,
            SyntaxKind::LeftBracket,
            "[",
            SyntaxKind::RightBracket,
            "]",
        )?;
        let mut content = lower_elements(node, &elements[1..elements.len() - 1])?;
        Ok(ParagraphElement::Reference(merge_tokens(
            &mut content,
            "reference content",
        )?))
    })
}

pub fn lower_legacy_section_reference(
    syntax: &SectionReferenceSyntax,
) -> Result<ParagraphElement, DiagnosticStore> {
    lower_value(syntax.syntax(), "section-reference", |node| {
        let elements = direct_elements(node)?;
        let Some(first) = elements.first() else {
            return Err(String::from(
                "section-reference syntax is missing its section sigil",
            ));
        };
        require_token(node, first, SyntaxKind::SectionSigil, "§")?;
        let mut content = lower_elements(node, &elements[1..])?;
        Ok(ParagraphElement::SectionReference(merge_tokens(
            &mut content,
            "section-reference content",
        )?))
    })
}

pub fn lower_legacy_paragraph_text(
    syntax: &ParagraphTextSyntax,
) -> Result<ParagraphElement, DiagnosticStore> {
    lower_value(syntax.syntax(), "paragraph-text", |node| {
        let elements = direct_elements(node)?;
        let mut content = lower_elements(node, &elements)?;
        let mut token = merge_tokens(&mut content, "paragraph-text content")?;
        token.kind = TokenKind::Text;
        Ok(ParagraphElement::Text(token))
    })
}

pub fn lower_legacy_thematic_break(
    syntax: &ThematicBreakSyntax,
) -> Result<SectionElement, DiagnosticStore> {
    lower_value(syntax.syntax(), "thematic-break", |node| {
        let elements = direct_elements(node)?;
        if elements.is_empty() {
            return Err(String::from(
                "thematic-break syntax requires an asterisk and newline",
            ));
        }
        let mut saw_asterisk = false;
        let mut saw_spacing = false;
        for (index, element) in elements.iter().enumerate() {
            let token = direct_token(element)
                .ok_or_else(|| String::from("thematic-break syntax cannot contain child nodes"))?;
            validate_token(token)?;
            let is_last = index + 1 == elements.len();
            if is_last {
                if !matches!(
                    token.kind(),
                    SyntaxKind::Newline | SyntaxKind::CarriageReturn
                ) {
                    return Err(String::from("thematic-break syntax must end in a newline"));
                }
                continue;
            }
            match token.kind() {
                SyntaxKind::Asterisk if !saw_spacing => saw_asterisk = true,
                SyntaxKind::Whitespace | SyntaxKind::Tab if saw_asterisk => {
                    saw_spacing = true;
                }
                _ => {
                    return Err(String::from(
                        "thematic-break syntax contains an unexpected token",
                    ));
                }
            }
        }
        if !saw_asterisk {
            return Err(String::from(
                "thematic-break syntax requires at least one asterisk",
            ));
        }
        Ok(SectionElement::ThematicBreak)
    })
}

pub fn lower_legacy_equation(syntax: &EquationSyntax) -> Result<LegacyToken, DiagnosticStore> {
    lower_value(syntax.syntax(), "equation", |node| {
        let elements = direct_elements(node)?;
        let Some(first) = elements.first() else {
            return Err(String::from("equation syntax is missing its opening sigil"));
        };
        require_token(node, first, SyntaxKind::EquationSigil, "$$")?;
        let mut content = lower_elements(node, &elements[1..])?;
        merge_tokens(&mut content, "equation content")
    })
}

fn lower_value<T>(
    syntax: &SyntaxNode,
    name: &str,
    lower: impl FnOnce(&SyntaxNode) -> Result<T, String>,
) -> Result<T, DiagnosticStore> {
    if syntax.flags().intersects(INVALID_NODE_FLAGS) {
        return Err(failure_store(
            syntax,
            name,
            alloc::format!("{name} compatibility lowering requires error-free syntax"),
        ));
    }
    lower(syntax).map_err(|message| failure_store(syntax, name, message))
}

fn direct_elements(syntax: &SyntaxNode) -> Result<Vec<SyntaxElement>, String> {
    Ok(syntax.children_with_tokens())
}

fn require_delimited(
    syntax: &SyntaxNode,
    elements: &[SyntaxElement],
    opening_kind: SyntaxKind,
    opening_text: &str,
    closing_kind: SyntaxKind,
    closing_text: &str,
) -> Result<(), String> {
    if elements.len() < 2 {
        return Err(String::from(
            "delimited syntax is missing an opening or closing delimiter",
        ));
    }
    require_token(syntax, &elements[0], opening_kind, opening_text)?;
    require_token(
        syntax,
        elements.last().expect("two elements have a last element"),
        closing_kind,
        closing_text,
    )
}

fn require_token(
    _syntax: &SyntaxNode,
    element: &SyntaxElement,
    kind: SyntaxKind,
    text: &str,
) -> Result<(), String> {
    let token =
        direct_token(element).ok_or_else(|| String::from("expected a direct delimiter token"))?;
    validate_token(token)?;
    if token.kind() != kind
        || token
            .text()
            .map_err(|_| String::from("cannot read delimiter source"))?
            != text
    {
        return Err(alloc::format!("expected {kind:?} delimiter {text:?}"));
    }
    Ok(())
}

fn direct_token(element: &SyntaxElement) -> Option<&SyntaxToken> {
    match element {
        SyntaxElement::Token(token) => Some(token),
        SyntaxElement::Node(_) => None,
    }
}

fn lower_elements(
    syntax: &SyntaxNode,
    elements: &[SyntaxElement],
) -> Result<Vec<LegacyToken>, String> {
    elements
        .iter()
        .map(|element| lower_element(syntax, element))
        .collect()
}

fn lower_element(syntax: &SyntaxNode, element: &SyntaxElement) -> Result<LegacyToken, String> {
    match element {
        SyntaxElement::Token(token) => lower_token(syntax, token),
        SyntaxElement::Node(node) if node.kind() == SyntaxKind::EscapedCharacter => {
            lower_escaped_character(node)
        }
        SyntaxElement::Node(_) => Err(String::from(
            "canonical Mechdown value contains an unexpected child node",
        )),
    }
}

fn lower_escaped_character(node: &SyntaxNode) -> Result<LegacyToken, String> {
    if node.flags().intersects(INVALID_NODE_FLAGS) {
        return Err(String::from("escaped-character content is not error-free"));
    }
    let elements = node.children_with_tokens();
    if elements.len() != 2 {
        return Err(String::from(
            "escaped-character content requires two tokens",
        ));
    }
    require_token(node, &elements[0], SyntaxKind::Backslash, "\\")?;
    let value = direct_token(&elements[1])
        .ok_or_else(|| String::from("escaped-character value must be a token"))?;
    validate_token(value)?;
    if value.kind() != SyntaxKind::EscapedChar {
        return Err(String::from(
            "escaped-character content has an invalid value token",
        ));
    }
    let text = value
        .text()
        .map_err(|_| String::from("cannot read escaped-character value"))?;
    let chars = text
        .chars()
        .map(|character| match character {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            other => other,
        })
        .collect();
    let src_range = source::source_range(node.source(), value.range())
        .ok_or_else(|| String::from("cannot convert escaped-character source range"))?;
    Ok(LegacyToken {
        kind: TokenKind::EscapedChar,
        chars,
        src_range,
    })
}

fn lower_token(syntax: &SyntaxNode, token: &SyntaxToken) -> Result<LegacyToken, String> {
    validate_token(token)?;
    let kind = legacy_kind(token.kind()).ok_or_else(|| {
        alloc::format!(
            "canonical Mechdown value contains unsupported token {:?}",
            token.kind()
        )
    })?;
    let text = token
        .text()
        .map_err(|_| String::from("cannot read canonical token source"))?;
    let src_range = source::source_range(syntax.source(), token.range())
        .ok_or_else(|| String::from("cannot convert canonical token source range"))?;
    Ok(LegacyToken {
        kind,
        chars: text.chars().collect(),
        src_range,
    })
}

fn validate_token(token: &SyntaxToken) -> Result<(), String> {
    if token.flags().intersects(INVALID_TOKEN_FLAGS) {
        return Err(String::from(
            "canonical Mechdown value contains a missing or error token",
        ));
    }
    Ok(())
}

fn merge_tokens(tokens: &mut Vec<LegacyToken>, description: &str) -> Result<LegacyToken, String> {
    LegacyToken::merge_tokens(tokens).ok_or_else(|| alloc::format!("{description} cannot be empty"))
}

fn legacy_kind(kind: SyntaxKind) -> Option<TokenKind> {
    Some(match kind {
        SyntaxKind::AbstractSigil => TokenKind::AbstractSigil,
        SyntaxKind::Alpha => TokenKind::Alpha,
        SyntaxKind::Ampersand => TokenKind::Ampersand,
        SyntaxKind::Any => TokenKind::Any,
        SyntaxKind::Apostrophe => TokenKind::Apostrophe,
        SyntaxKind::Asterisk => TokenKind::Asterisk,
        SyntaxKind::AssignOperator => TokenKind::AssignOperator,
        SyntaxKind::AsyncTransitionOperator => TokenKind::AsyncTransitionOperator,
        SyntaxKind::At => TokenKind::At,
        SyntaxKind::Backslash => TokenKind::Backslash,
        SyntaxKind::Bar => TokenKind::Bar,
        SyntaxKind::BoxDrawing => TokenKind::BoxDrawing,
        SyntaxKind::Caret => TokenKind::Caret,
        SyntaxKind::CarriageReturn => TokenKind::CarriageReturn,
        SyntaxKind::Colon => TokenKind::Colon,
        SyntaxKind::Comma => TokenKind::Comma,
        SyntaxKind::Dash => TokenKind::Dash,
        SyntaxKind::DefineOperatorToken => TokenKind::DefineOperator,
        SyntaxKind::Digit => TokenKind::Digit,
        SyntaxKind::Dollar => TokenKind::Dollar,
        SyntaxKind::Emoji => TokenKind::Emoji,
        SyntaxKind::EmphasisSigil => TokenKind::EmphasisSigil,
        SyntaxKind::EquationSigil => TokenKind::EquationSigil,
        SyntaxKind::Equal => TokenKind::Equal,
        SyntaxKind::ErrorSigil => TokenKind::ErrorSigil,
        SyntaxKind::EscapedChar => TokenKind::EscapedChar,
        SyntaxKind::Exclamation => TokenKind::Exclamation,
        SyntaxKind::False => TokenKind::False,
        SyntaxKind::FloatLeft => TokenKind::FloatLeft,
        SyntaxKind::FloatRight => TokenKind::FloatRight,
        SyntaxKind::FootnotePrefix => TokenKind::FootnotePrefix,
        SyntaxKind::GenOperator => TokenKind::GenOperator,
        SyntaxKind::GeneratorArrow => TokenKind::GeneratorArrow,
        SyntaxKind::Grave => TokenKind::Grave,
        SyntaxKind::GraveCodeBlockSigil => TokenKind::GraveCodeBlockSigil,
        SyntaxKind::HashTag => TokenKind::HashTag,
        SyntaxKind::HighlightSigil => TokenKind::HighlightSigil,
        SyntaxKind::HttpPrefix => TokenKind::HttpPrefix,
        SyntaxKind::IdeaSigil => TokenKind::IdeaSigil,
        SyntaxKind::ImgPrefix => TokenKind::ImgPrefix,
        SyntaxKind::InfoSigil => TokenKind::InfoSigil,
        SyntaxKind::LeftAngle => TokenKind::LeftAngle,
        SyntaxKind::LeftBrace => TokenKind::LeftBrace,
        SyntaxKind::LeftBracket => TokenKind::LeftBracket,
        SyntaxKind::LeftParen => TokenKind::LeftParenthesis,
        SyntaxKind::MikaSectionOpen => TokenKind::MikaSectionOpen,
        SyntaxKind::MikaSectionClose => TokenKind::MikaSectionClose,
        SyntaxKind::ModuleExportSigil => TokenKind::ModuleExportSigil,
        SyntaxKind::ModuleImportSigil => TokenKind::ModuleImportSigil,
        SyntaxKind::Newline => TokenKind::Newline,
        SyntaxKind::Not => TokenKind::Not,
        SyntaxKind::OutputOperator => TokenKind::OutputOperator,
        SyntaxKind::Percent => TokenKind::Percent,
        SyntaxKind::Period => TokenKind::Period,
        SyntaxKind::Plus => TokenKind::Plus,
        SyntaxKind::PromptSigil => TokenKind::PromptSigil,
        SyntaxKind::Question => TokenKind::Question,
        SyntaxKind::QuestionSigil => TokenKind::QuestionSigil,
        SyntaxKind::Quote => TokenKind::Quote,
        SyntaxKind::QuoteSigil => TokenKind::QuoteSigil,
        SyntaxKind::RightAngle => TokenKind::RightAngle,
        SyntaxKind::RightBrace => TokenKind::RightBrace,
        SyntaxKind::RightBracket => TokenKind::RightBracket,
        SyntaxKind::RightParen => TokenKind::RightParenthesis,
        SyntaxKind::SectionSigil => TokenKind::SectionSigil,
        SyntaxKind::Semicolon => TokenKind::Semicolon,
        SyntaxKind::Slash => TokenKind::Slash,
        SyntaxKind::SpreadOperator => TokenKind::SpreadOperator,
        SyntaxKind::StrikeSigil => TokenKind::StrikeSigil,
        SyntaxKind::StrongSigil => TokenKind::StrongSigil,
        SyntaxKind::SuccessSigil => TokenKind::SuccessSigil,
        SyntaxKind::SynthOperator => TokenKind::SynthOperator,
        SyntaxKind::Tab => TokenKind::Tab,
        SyntaxKind::Text => TokenKind::Text,
        SyntaxKind::Tilde => TokenKind::Tilde,
        SyntaxKind::TildeCodeBlockSigil => TokenKind::TildeCodeBlockSigil,
        SyntaxKind::TransitionOperator => TokenKind::TransitionOperator,
        SyntaxKind::True => TokenKind::True,
        SyntaxKind::UnderlineSigil => TokenKind::UnderlineSigil,
        SyntaxKind::Underscore => TokenKind::Underscore,
        SyntaxKind::WarningSigil => TokenKind::WarningSigil,
        SyntaxKind::Whitespace => TokenKind::Space,
        _ => return None,
    })
}

fn failure_store(syntax: &SyntaxNode, name: &str, message: String) -> DiagnosticStore {
    let mut ids = IdGenerator::new();
    let mut diagnostics = DiagnosticStore::new(syntax.source().revision());
    diagnostics.push(Diagnostic {
        id: ids.diagnostic(),
        code: DiagnosticCode::from(alloc::format!("lowering/invalid-{name}-syntax").as_str()),
        phase: DiagnosticPhase::Lowering,
        severity: Severity::Error,
        rule: None,
        context: None,
        primary: DiagnosticAnchor::Absolute {
            revision: syntax.source().revision(),
            range: syntax.range(),
        },
        labels: Vec::new(),
        expected: Vec::new(),
        found: None,
        fixes: Vec::new(),
        related: Vec::new(),
        recovery: None,
        tags: DiagnosticTags::NONE,
        message,
    });
    diagnostics
}
