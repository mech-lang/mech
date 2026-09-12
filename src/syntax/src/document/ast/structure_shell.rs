//! Typed syntax views for the closed Phase 2H structure shell.

use alloc::vec::Vec;

use crate::document::ast::literals::EmptyLiteralSyntax;
use crate::document::{AstNode, SyntaxKind, SyntaxNode, SyntaxToken};

macro_rules! structure_shell_ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Clone, Debug)]
        pub struct $name(pub(crate) SyntaxNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                kind == SyntaxKind::$kind
            }

            fn cast(syntax: SyntaxNode) -> Option<Self> {
                Self::can_cast(syntax.kind()).then_some(Self(syntax))
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

/// The exact source spelling used by a future complete matrix's delimiter.
///
/// This decodes one physical syntax token. It intentionally does not infer a
/// layout or require an opening and closing delimiter to agree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatrixDelimiterStyle {
    Bracket,
    RoundedBox,
    LightBox,
    BoldBox,
}

impl MatrixDelimiterStyle {
    /// Decode an exact opening delimiter token.
    pub fn from_opening_token(token: &SyntaxToken) -> Option<Self> {
        let text = token.text().ok()?;
        match (token.kind(), text.as_str()) {
            (SyntaxKind::LeftBracket, "[") => Some(Self::Bracket),
            (SyntaxKind::BoxDrawing, "╭") => Some(Self::RoundedBox),
            (SyntaxKind::BoxDrawing, "┌") => Some(Self::LightBox),
            (SyntaxKind::BoxDrawing, "┏") => Some(Self::BoldBox),
            _ => None,
        }
    }

    /// Decode an exact closing delimiter token.
    pub fn from_closing_token(token: &SyntaxToken) -> Option<Self> {
        let text = token.text().ok()?;
        match (token.kind(), text.as_str()) {
            (SyntaxKind::RightBracket, "]") => Some(Self::Bracket),
            (SyntaxKind::BoxDrawing, "╯") => Some(Self::RoundedBox),
            (SyntaxKind::BoxDrawing, "┘") => Some(Self::LightBox),
            (SyntaxKind::BoxDrawing, "┛") => Some(Self::BoldBox),
            _ => None,
        }
    }

    pub const fn opening_text(self) -> &'static str {
        match self {
            Self::Bracket => "[",
            Self::RoundedBox => "╭",
            Self::LightBox => "┌",
            Self::BoldBox => "┏",
        }
    }

    pub const fn closing_text(self) -> &'static str {
        match self {
            Self::Bracket => "]",
            Self::RoundedBox => "╯",
            Self::LightBox => "┘",
            Self::BoldBox => "┛",
        }
    }
}

structure_shell_ast_node!(TableRowSeparatorSyntax, TableRowSeparator);
structure_shell_ast_node!(EmptyMapSyntax, EmptyMap);
structure_shell_ast_node!(EmptySetSyntax, EmptySet);

impl TableRowSeparatorSyntax {
    /// Every physical token in source order, including retained horizontal
    /// trivia and any table-end spelling selected by the direct grammar.
    pub fn physical_tokens(&self) -> Vec<SyntaxToken> {
        self.0.tokens()
    }
}

impl EmptySetSyntax {
    /// The optional `_`-sequence retained by this concrete empty-set spelling.
    pub fn marker(&self) -> Option<EmptyLiteralSyntax> {
        self.0.children().find_map(EmptyLiteralSyntax::cast)
    }

    pub fn uses_explicit_empty_marker(&self) -> bool {
        self.marker().is_some()
    }
}
