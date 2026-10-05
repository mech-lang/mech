//! Source coordinates and token annotations shared by diagnostics.
//!
//! These describe retained-source positions; they are not an executable AST.

use crate::*;
use core::cmp::Ordering;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Ord)]
pub struct SourceLocation {
    pub row: usize,
    pub col: usize,
}

impl PartialOrd for SourceLocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.row < other.row {
            Some(Ordering::Less)
        } else if self.row > other.row {
            Some(Ordering::Greater)
        } else {
            self.col.partial_cmp(&other.col)
        }
    }
}

impl fmt::Debug for SourceLocation {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.row, self.col)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct SourceRange {
    pub start: SourceLocation,
    pub end: SourceLocation,
}

/// Coordinates in SourceRange are 1-indexed, i.e. they directly translate
/// human's view to line and column numbers.  Having value 0 means the
/// range is not initialized.
impl Default for SourceRange {
    fn default() -> Self {
        SourceRange {
            start: SourceLocation { row: 0, col: 0 },
            end: SourceLocation { row: 0, col: 0 },
        }
    }
}

impl fmt::Debug for SourceRange {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{:?}, {:?})", self.start, self.end)
    }
}

pub fn merge_src_range(r1: SourceRange, r2: SourceRange) -> SourceRange {
    SourceRange {
        start: r1.start.min(r2.start),
        end: r2.end.max(r2.end),
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    AbstractSigil,
    Alpha,
    Ampersand,
    Any,
    Apostrophe,
    AssignOperator,
    Asterisk,
    AsyncTransitionOperator,
    At,
    Backslash,
    Bar,
    BoxDrawing,
    Caret,
    CarriageReturn,
    CarriageReturnNewLine,
    Colon,
    CodeBlock,
    Comma,
    Dash,
    DefineOperator,
    Digit,
    Dollar,
    Emoji,
    EmphasisSigil,
    Empty,
    Equal,
    EquationSigil,
    Error,
    ErrorSigil,
    EscapedChar,
    Exclamation,
    False,
    FloatLeft,
    FloatRight,
    FootnotePrefix,
    GenOperator,
    GeneratorArrow,
    Grave,
    GraveCodeBlockSigil,
    HashTag,
    HighlightSigil,
    HttpPrefix,
    IdeaSigil,
    Identifier,
    ImgPrefix,
    InfoSigil,
    InlineCode,
    LeftAngle,
    LeftBrace,
    LeftBracket,
    LeftParenthesis,
    #[cfg(feature = "mika")]
    Mika(Mika),
    MikaSection,
    MikaSectionOpen,
    MikaSectionClose,
    ModuleExportSigil,
    ModuleImportSigil,
    Newline,
    Not,
    Number,
    OutputOperator,
    Percent,
    Period,
    Plus,
    PromptSigil,
    Question,
    QuestionSigil,
    Quote,
    QuoteSigil,
    RightAngle,
    RightBrace,
    RightBracket,
    RightParenthesis,
    SectionSigil,
    Semicolon,
    Space,
    SpreadOperator,
    Slash,
    String,
    StrikeSigil,
    StrongSigil,
    SuccessSigil,
    SynthOperator,
    Tab,
    Text,
    Tilde,
    TildeCodeBlockSigil,
    Title,
    TransitionOperator,
    True,
    UnderlineSigil,
    Underscore,
    WarningSigil,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub chars: Vec<char>,
    pub src_range: SourceRange,
}

impl fmt::Debug for Token {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}:{:?}:{:?}",
            self.kind,
            String::from_iter(self.chars.iter().cloned()),
            self.src_range
        )
    }
}

impl Default for Token {
    fn default() -> Self {
        Token {
            kind: TokenKind::Empty,
            chars: vec![],
            src_range: SourceRange::default(),
        }
    }
}

impl Token {
    pub fn new(kind: TokenKind, src_range: SourceRange, chars: Vec<char>) -> Token {
        Token {
            kind,
            chars,
            src_range,
        }
    }

    pub fn to_string(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn merge_tokens(tokens: &mut Vec<Token>) -> Option<Token> {
        if tokens.len() == 0 {
            None
        } else if tokens.len() == 1 {
            Some(tokens[0].clone())
        } else {
            let first = tokens[0].src_range.clone();
            let kind = tokens[0].kind.clone();
            let last = tokens.last().unwrap().src_range.clone();
            let src_range = merge_src_range(first, last);
            let chars: Vec<char> = tokens.iter_mut().fold(vec![], |mut m, ref mut t| {
                m.append(&mut t.chars.clone());
                m
            });
            let merged_token = Token {
                kind,
                chars,
                src_range,
            };
            Some(merged_token)
        }
    }
}
