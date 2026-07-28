//! Manifest-driven grammar conformance tests.
//!
//! Phase 0 treats the current parser as law.  The fixtures deliberately cover
//! complete public roots, lower-level prefix contracts, and recovery as three
//! different outcomes.  Snapshots are ordinary JSON; source locations and
//! diagnostic prose are removed while token text, variants, ordering, and AST
//! shape remain.

#![cfg(feature = "serde")]

use mech_core::{FloatDirection, OpAssignOp, Token, TokenKind};
#[cfg(feature = "mika")]
use mech_core::{MikaArm, MikaEyeLeft, MikaEyeRight, MikaNose};
use mech_syntax::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use unicode_segmentation::UnicodeSegmentation;

const HEADER: &[&str] = &[
    "id",
    "rule",
    "entry-point",
    "feature-set",
    "expected-result",
    "source-file",
    "snapshot-file",
    "notes",
];

const EXPECTED_RESULTS: &[&str] = &[
    "accept",
    "reject",
    "accept-prefix",
    "recover-with-error",
    "feature-disabled",
];

const ENTRY_POINTS: &[&str] = &[
    "abstract-el",
    "activation-scope",
    "alternative-terminal-contracts",
    "body",
    "boolean",
    "box-drawing-char",
    "box-drawing-emoji",
    "citation",
    "code-block",
    "comment",
    "emoji",
    "equation",
    "error-block",
    "escaped-char",
    "eval-inline-mech-code",
    "expression",
    "fancy-table",
    "figures",
    "float",
    "footnote",
    "forbidden-emoji",
    "fsm-async-transition",
    "fsm-block-transition",
    "fsm-comment-arm",
    "fsm-declare",
    "fsm-implementation",
    "fsm-instance",
    "fsm-output",
    "fsm-specification",
    "fsm-state-transition",
    "fsm-statement-transition",
    "fsm-transition",
    "function-define",
    "gen-operator",
    "grammar",
    "grouping-symbol",
    "identifier",
    "idea-block",
    "image",
    "info-block",
    "inline-mech-code",
    "kind-annotation",
    "literal",
    "match-expression",
    "mech-code",
    "mechdown-list",
    "mechdown-table",
    "mika",
    "mika-alternative-terminal-contracts",
    "mika-expression-inner",
    "mika-eye-left",
    "mika-eye-right",
    "mini-mika",
    "module-import",
    "newline-indent",
    "normalized-citation",
    "normalized-footnote",
    "normalized-paragraph-newline",
    "number",
    "paragraph",
    "paragraph-newline",
    "paragraph-text",
    "parse",
    "pattern",
    "prompt",
    "question-block",
    "quote-block",
    "regular-table",
    "repl",
    "repl-crlf",
    "section",
    "section-element",
    "slice",
    "space-tab1",
    "statement",
    "strike-sigil",
    "string",
    "structure",
    "subscript",
    "subtitle",
    "success-block",
    "synth-operator",
    "table-column",
    "table-horz",
    "terminal-contracts",
    "thematic-break",
    "title",
    "ul-subtitle",
    "underline-sigil",
    "warning-block",
    "xor-operator",
];

#[derive(Clone, Debug)]
struct Case {
    id: String,
    rule: String,
    entry_point: String,
    feature_set: String,
    expected_result: String,
    source_file: String,
    snapshot_file: String,
    notes: String,
}

#[derive(Debug)]
enum ParseOutcome {
    Success {
        ast: Value,
        consumed: usize,
        source_len: usize,
        remaining: String,
        diagnostics: usize,
    },
    Failure,
}

type TerminalParser = for<'source> fn(ParseString<'source>) -> ParseResult<'source, Token>;

#[derive(Clone, Copy)]
struct FixedTerminalContract {
    production_id: &'static str,
    function_name: &'static str,
    parser: TerminalParser,
    literal: &'static str,
    literal_source: &'static str,
    expected_kind: TokenKind,
    kind_name: &'static str,
    parser_macro: &'static str,
}

macro_rules! terminal_contract {
    ($id:literal, $function:ident, $literal:literal, $kind:ident, $parser_macro:ident) => {
        FixedTerminalContract {
            production_id: $id,
            function_name: stringify!($function),
            parser: $function,
            literal: $literal,
            literal_source: stringify!($literal),
            expected_kind: TokenKind::$kind,
            kind_name: stringify!($kind),
            parser_macro: stringify!($parser_macro),
        }
    };
}

const FIXED_TERMINAL_CONTRACTS: &[FixedTerminalContract] = &[
    terminal_contract!("base.ampersand", ampersand, "&", Ampersand, leaf),
    terminal_contract!("base.apostrophe", apostrophe, "'", Apostrophe, leaf),
    terminal_contract!("base.asterisk", asterisk, "*", Asterisk, leaf),
    terminal_contract!("base.at", at, "@", At, leaf),
    terminal_contract!("base.bar", bar, "|", Bar, leaf),
    terminal_contract!("base.backslash", backslash, "\\", Backslash, leaf),
    terminal_contract!("base.caret", caret, "^", Caret, leaf),
    terminal_contract!("base.colon", colon, ":", Colon, leaf),
    terminal_contract!("base.comma", comma, ",", Comma, leaf),
    terminal_contract!("base.dash", dash, "-", Dash, leaf),
    terminal_contract!("base.dollar", dollar, "$", Dollar, leaf),
    terminal_contract!("base.equal", equal, "=", Equal, leaf),
    terminal_contract!("base.exclamation", exclamation, "!", Exclamation, leaf),
    terminal_contract!("base.grave", grave, "`", Grave, leaf),
    terminal_contract!("base.hashtag", hashtag, "#", HashTag, leaf),
    terminal_contract!("base.negate", negate, "¬", Not, leaf),
    terminal_contract!("base.percent", percent, "%", Percent, leaf),
    terminal_contract!("base.period", period, ".", Period, leaf),
    terminal_contract!("base.plus", plus, "+", Plus, leaf),
    terminal_contract!("base.question", question, "?", Question, leaf),
    terminal_contract!("base.quote", quote, "\"", Quote, leaf),
    terminal_contract!("base.semicolon", semicolon, ";", Semicolon, leaf),
    terminal_contract!("base.slash", slash, "/", Slash, leaf),
    terminal_contract!("base.tilde", tilde, "~", Tilde, leaf),
    terminal_contract!("base.underscore", underscore, "_", Underscore, leaf),
    terminal_contract!("base.check-mark", check_mark, "✓", True, leaf),
    terminal_contract!("base.cross", cross, "✗", False, leaf),
    terminal_contract!(
        "base.english-true-literal",
        english_true_literal,
        "true",
        True,
        leaf
    ),
    terminal_contract!(
        "base.english-false-literal",
        english_false_literal,
        "false",
        False,
        leaf
    ),
    terminal_contract!("base.space", space, " ", Space, leaf),
    terminal_contract!("base.nbsp", nbsp, "\u{00A0}", Space, leaf),
    terminal_contract!("base.thin-space", thin_space, "\u{2009}", Space, leaf),
    terminal_contract!("base.new-line-char", new_line_char, "\n", Newline, leaf),
    terminal_contract!(
        "base.carriage-return",
        carriage_return,
        "\r",
        CarriageReturn,
        leaf
    ),
    terminal_contract!(
        "base.carriage-return-new-line",
        carriage_return_new_line,
        "\r\n",
        CarriageReturn,
        leaf
    ),
    terminal_contract!("base.tab", tab, "\t", Tab, leaf),
    terminal_contract!("base.left-bracket", left_bracket, "[", LeftBracket, leaf),
    terminal_contract!(
        "base.left-parenthesis",
        left_parenthesis,
        "(",
        LeftParenthesis,
        leaf
    ),
    terminal_contract!("base.left-brace", left_brace, "{", LeftBrace, leaf),
    terminal_contract!("base.left-angle1", left_angle1, "<", LeftAngle, leaf),
    terminal_contract!("base.left-angle2", left_angle2, "⟨", LeftAngle, leaf),
    terminal_contract!("base.right-bracket", right_bracket, "]", RightBracket, leaf),
    terminal_contract!(
        "base.right-parenthesis",
        right_parenthesis,
        ")",
        RightParenthesis,
        leaf
    ),
    terminal_contract!("base.right-brace", right_brace, "}", RightBrace, leaf),
    terminal_contract!("base.right-angle1", right_angle1, ">", RightAngle, leaf),
    terminal_contract!("base.right-angle2", right_angle2, "⟩", RightAngle, leaf),
    terminal_contract!("base.box-tl-round", box_tl_round, "╭", BoxDrawing, leaf),
    terminal_contract!("base.box-tr-round", box_tr_round, "╮", BoxDrawing, leaf),
    terminal_contract!("base.box-bl-round", box_bl_round, "╰", BoxDrawing, leaf),
    terminal_contract!("base.box-br-round", box_br_round, "╯", BoxDrawing, leaf),
    terminal_contract!("base.box-tl-bold", box_tl_bold, "┏", BoxDrawing, leaf),
    terminal_contract!("base.box-tr-bold", box_tr_bold, "┓", BoxDrawing, leaf),
    terminal_contract!("base.box-bl-bold", box_bl_bold, "┗", BoxDrawing, leaf),
    terminal_contract!("base.box-br-bold", box_br_bold, "┛", BoxDrawing, leaf),
    terminal_contract!("base.box-tl", box_tl, "┌", BoxDrawing, leaf),
    terminal_contract!("base.box-tr", box_tr, "┐", BoxDrawing, leaf),
    terminal_contract!("base.box-bl", box_bl, "└", BoxDrawing, leaf),
    terminal_contract!("base.box-br", box_br, "┘", BoxDrawing, leaf),
    terminal_contract!("base.box-cross", box_cross, "┼", BoxDrawing, leaf),
    terminal_contract!("base.box-horz", box_horz, "─", BoxDrawing, leaf),
    terminal_contract!("base.box-t-left", box_t_left, "├", BoxDrawing, leaf),
    terminal_contract!("base.box-t-right", box_t_right, "┤", BoxDrawing, leaf),
    terminal_contract!("base.box-t-top", box_t_top, "┬", BoxDrawing, leaf),
    terminal_contract!("base.box-t-bottom", box_t_bottom, "┴", BoxDrawing, leaf),
    terminal_contract!("base.box-vert", box_vert, "│", BoxDrawing, leaf),
    terminal_contract!("base.box-vert-bold", box_vert_bold, "┃", BoxDrawing, leaf),
    terminal_contract!(
        "base.abstract-sigil",
        abstract_sigil,
        "%%",
        AbstractSigil,
        leaf
    ),
    terminal_contract!(
        "base.emphasis-sigil",
        emphasis_sigil,
        "*",
        EmphasisSigil,
        leaf
    ),
    terminal_contract!(
        "base.equation-sigil",
        equation_sigil,
        "$$",
        EquationSigil,
        leaf
    ),
    terminal_contract!(
        "base.footnote-prefix",
        footnote_prefix,
        "[^",
        FootnotePrefix,
        leaf
    ),
    terminal_contract!("base.float-left", float_left, "<<:", FloatLeft, leaf),
    terminal_contract!("base.float-right", float_right, ":>>", FloatRight, leaf),
    terminal_contract!("base.http-prefix", http_prefix, "http", HttpPrefix, leaf),
    terminal_contract!(
        "base.highlight-sigil",
        highlight_sigil,
        "!!",
        HighlightSigil,
        leaf
    ),
    terminal_contract!("base.img-prefix", img_prefix, "![", ImgPrefix, leaf),
    terminal_contract!("base.quote-sigil", quote_sigil, ">", QuoteSigil, leaf),
    terminal_contract!(
        "base.question-sigil",
        question_sigil,
        "(?)>",
        QuestionSigil,
        leaf
    ),
    terminal_contract!("base.info-sigil", info_sigil, "(i)>", InfoSigil, leaf),
    terminal_contract!("base.idea-sigil", idea_sigil, "(*)>", IdeaSigil, leaf),
    terminal_contract!(
        "base.warning-sigil",
        warning_sigil,
        "(!)>",
        WarningSigil,
        leaf
    ),
    terminal_contract!("base.error-sigil", error_sigil, "(x)>", ErrorSigil, leaf),
    terminal_contract!(
        "base.error-alt-sigil",
        error_alt_sigil,
        "(✗)>",
        ErrorSigil,
        leaf
    ),
    terminal_contract!(
        "base.success-check-sigil",
        success_check_sigil,
        "(✓)>",
        SuccessSigil,
        leaf
    ),
    terminal_contract!(
        "base.success-sigil",
        success_sigil,
        "(+)>",
        SuccessSigil,
        leaf
    ),
    terminal_contract!("base.strike-sigil", strike_sigil, "~~", StrikeSigil, leaf),
    terminal_contract!("base.strong-sigil", strong_sigil, "**", StrongSigil, leaf),
    terminal_contract!(
        "base.grave-codeblock-sigil",
        grave_codeblock_sigil,
        "```",
        GraveCodeBlockSigil,
        leaf
    ),
    terminal_contract!(
        "base.tilde-codeblock-sigil",
        tilde_codeblock_sigil,
        "~~~",
        TildeCodeBlockSigil,
        leaf
    ),
    terminal_contract!(
        "base.underline-sigil",
        underline_sigil,
        "__",
        UnderlineSigil,
        leaf
    ),
    terminal_contract!("base.section-sigil", section_sigil, "§", SectionSigil, leaf),
    terminal_contract!(
        "base.mika-section-open",
        mika_section_open,
        "⸢",
        MikaSectionOpen,
        leaf
    ),
    terminal_contract!(
        "base.mika-section-close",
        mika_section_close,
        "⸥",
        MikaSectionClose,
        leaf
    ),
    terminal_contract!("base.prompt-sigil", prompt_sigil, ">:", PromptSigil, leaf),
    terminal_contract!(
        "base.module-import-sigil",
        module_import_sigil,
        "+>",
        ModuleImportSigil,
        leaf
    ),
    terminal_contract!(
        "base.module-export-sigil",
        module_export_sigil,
        "<+",
        ModuleExportSigil,
        leaf
    ),
    terminal_contract!(
        "base.assign-operator",
        assign_operator,
        "=",
        AssignOperator,
        ws0_leaf
    ),
    terminal_contract!(
        "base.async-transition-operator",
        async_transition_operator,
        "~>",
        AsyncTransitionOperator,
        ws0_leaf
    ),
    terminal_contract!(
        "base.define-operator",
        define_operator,
        ":=",
        DefineOperator,
        ws0_leaf
    ),
    terminal_contract!(
        "base.synth-operator",
        synth_operator,
        "?=",
        SynthOperator,
        ws0_leaf
    ),
    terminal_contract!(
        "base.gen-operator",
        gen_operator,
        "@=",
        GenOperator,
        ws0_leaf
    ),
    terminal_contract!(
        "base.output-operator-a",
        output_operator_a,
        "=>",
        OutputOperator,
        ws0_leaf
    ),
    terminal_contract!(
        "base.output-operator-u",
        output_operator_u,
        "⇒",
        OutputOperator,
        ws0_leaf
    ),
    terminal_contract!(
        "base.transition-operator-a",
        transition_operator_a,
        "->",
        TransitionOperator,
        ws0_leaf
    ),
    terminal_contract!(
        "base.transition-operator-u",
        transition_operator_u,
        "→",
        TransitionOperator,
        ws0_leaf
    ),
    terminal_contract!(
        "base.generator-arrow",
        generator_arrow,
        "<-",
        GeneratorArrow,
        ws0_leaf
    ),
    terminal_contract!(
        "base.generator-arrow-u",
        generator_arrow_u,
        "←",
        GeneratorArrow,
        ws0_leaf
    ),
    terminal_contract!(
        "base.spread-operator-a",
        spread_operator_a,
        "...",
        SpreadOperator,
        ws0_leaf
    ),
    terminal_contract!(
        "base.spread-operator-u",
        spread_operator_u,
        "…",
        SpreadOperator,
        ws0_leaf
    ),
];

#[derive(Clone, Copy)]
struct AlternativeTokenContract {
    wrapper: &'static str,
    direct_branch: &'static str,
    leaf_branch: &'static str,
    parser: TerminalParser,
    literal: &'static str,
    expected_kind: TokenKind,
    expected_text: &'static str,
    surrounded_whitespace: bool,
}

macro_rules! token_alt {
    ($wrapper:literal, $direct:literal, $leaf:literal, $parser:ident, $literal:literal, $kind:ident) => {
        AlternativeTokenContract {
            wrapper: $wrapper,
            direct_branch: $direct,
            leaf_branch: $leaf,
            parser: $parser,
            literal: $literal,
            expected_kind: TokenKind::$kind,
            expected_text: $literal,
            surrounded_whitespace: false,
        }
    };
}

macro_rules! token_alt_ws {
    ($wrapper:literal, $direct:literal, $leaf:literal, $parser:ident, $literal:literal, $kind:ident) => {
        AlternativeTokenContract {
            surrounded_whitespace: true,
            ..token_alt!($wrapper, $direct, $leaf, $parser, $literal, $kind)
        }
    };
}

macro_rules! token_alt_text {
    ($wrapper:literal, $direct:literal, $leaf:literal, $parser:ident, $literal:literal, $kind:ident, $text:literal) => {
        AlternativeTokenContract {
            expected_text: $text,
            ..token_alt!($wrapper, $direct, $leaf, $parser, $literal, $kind)
        }
    };
}

const ALTERNATIVE_TOKEN_CONTRACTS: &[AlternativeTokenContract] = &[
    token_alt_ws!(
        "transition_operator",
        "transition_operator_a",
        "transition_operator_a",
        transition_operator,
        "->",
        TransitionOperator
    ),
    token_alt_ws!(
        "transition_operator",
        "transition_operator_u",
        "transition_operator_u",
        transition_operator,
        "→",
        TransitionOperator
    ),
    token_alt_ws!(
        "output_operator",
        "output_operator_a",
        "output_operator_a",
        output_operator,
        "=>",
        OutputOperator
    ),
    token_alt_ws!(
        "output_operator",
        "output_operator_u",
        "output_operator_u",
        output_operator,
        "⇒",
        OutputOperator
    ),
    token_alt!(
        "grouping_symbol",
        "left_parenthesis",
        "left_parenthesis",
        grouping_symbol,
        "(",
        LeftParenthesis
    ),
    token_alt!(
        "grouping_symbol",
        "right_parenthesis",
        "right_parenthesis",
        grouping_symbol,
        ")",
        RightParenthesis
    ),
    token_alt!(
        "grouping_symbol",
        "left_angle",
        "left_angle1",
        grouping_symbol,
        "<",
        LeftAngle
    ),
    token_alt!(
        "grouping_symbol",
        "left_angle",
        "left_angle2",
        grouping_symbol,
        "⟨",
        LeftAngle
    ),
    token_alt!(
        "grouping_symbol",
        "right_angle",
        "right_angle1",
        grouping_symbol,
        ">",
        RightAngle
    ),
    token_alt!(
        "left_angle",
        "left_angle1",
        "left_angle1",
        left_angle,
        "<",
        LeftAngle
    ),
    token_alt!(
        "left_angle",
        "left_angle2",
        "left_angle2",
        left_angle,
        "⟨",
        LeftAngle
    ),
    token_alt!(
        "right_angle",
        "right_angle1",
        "right_angle1",
        right_angle,
        ">",
        RightAngle
    ),
    token_alt!(
        "right_angle",
        "right_angle2",
        "right_angle2",
        right_angle,
        "⟩",
        RightAngle
    ),
    token_alt!(
        "grouping_symbol",
        "right_angle",
        "right_angle2",
        grouping_symbol,
        "⟩",
        RightAngle
    ),
    token_alt!(
        "grouping_symbol",
        "left_brace",
        "left_brace",
        grouping_symbol,
        "{",
        LeftBrace
    ),
    token_alt!(
        "grouping_symbol",
        "right_brace",
        "right_brace",
        grouping_symbol,
        "}",
        RightBrace
    ),
    token_alt!(
        "grouping_symbol",
        "left_bracket",
        "left_bracket",
        grouping_symbol,
        "[",
        LeftBracket
    ),
    token_alt!(
        "grouping_symbol",
        "right_bracket",
        "right_bracket",
        grouping_symbol,
        "]",
        RightBracket
    ),
    token_alt!("punctuation", "period", "period", punctuation, ".", Period),
    token_alt!(
        "punctuation",
        "exclamation",
        "exclamation",
        punctuation,
        "!",
        Exclamation
    ),
    token_alt!(
        "punctuation",
        "question",
        "question",
        punctuation,
        "?",
        Question
    ),
    token_alt!("punctuation", "comma", "comma", punctuation, ",", Comma),
    token_alt!("punctuation", "colon", "colon", punctuation, ":", Colon),
    token_alt!(
        "punctuation",
        "semicolon",
        "semicolon",
        punctuation,
        ";",
        Semicolon
    ),
    token_alt!("punctuation", "quote", "quote", punctuation, "\"", Quote),
    token_alt!(
        "punctuation",
        "apostrophe",
        "apostrophe",
        punctuation,
        "'",
        Apostrophe
    ),
    token_alt!("symbol", "ampersand", "ampersand", symbol, "&", Ampersand),
    token_alt!("symbol", "grave", "grave", symbol, "`", Grave),
    token_alt!("symbol", "dollar", "dollar", symbol, "$", Dollar),
    token_alt!("symbol", "bar", "bar", symbol, "|", Bar),
    token_alt!("symbol", "percent", "percent", symbol, "%", Percent),
    token_alt!("symbol", "at", "at", symbol, "@", At),
    token_alt!("symbol", "slash", "slash", symbol, "/", Slash),
    token_alt!("symbol", "hashtag", "hashtag", symbol, "#", HashTag),
    token_alt!("symbol", "equal", "equal", symbol, "=", Equal),
    token_alt!("symbol", "backslash", "backslash", symbol, "\\", Backslash),
    token_alt!("symbol", "tilde", "tilde", symbol, "~", Tilde),
    token_alt!("symbol", "plus", "plus", symbol, "+", Plus),
    token_alt!("symbol", "dash", "dash", symbol, "-", Dash),
    token_alt!("symbol", "asterisk", "asterisk", symbol, "*", Asterisk),
    token_alt!("symbol", "caret", "caret", symbol, "^", Caret),
    token_alt!(
        "symbol",
        "underscore",
        "underscore",
        symbol,
        "_",
        Underscore
    ),
    token_alt!(
        "new_line",
        "carriage_return_new_line",
        "carriage_return_new_line",
        new_line,
        "\r\n",
        CarriageReturn
    ),
    token_alt!(
        "new_line",
        "new_line_char",
        "new_line_char",
        new_line,
        "\n",
        Newline
    ),
    token_alt!(
        "new_line",
        "carriage_return",
        "carriage_return",
        new_line,
        "\r",
        CarriageReturn
    ),
    token_alt!("space_tab", "space", "space", space_tab, " ", Space),
    token_alt!("space_tab", "tab", "tab", space_tab, "\t", Tab),
    token_alt!("space_tab", "nbsp", "nbsp", space_tab, "\u{00A0}", Space),
    token_alt!(
        "space_tab",
        "thin_space",
        "thin_space",
        space_tab,
        "\u{2009}",
        Space
    ),
    token_alt_text!(
        "escaped_char",
        "alpha_token",
        "alpha:n",
        escaped_char,
        "\\n",
        EscapedChar,
        "\n"
    ),
    token_alt_text!(
        "escaped_char",
        "alpha_token",
        "alpha:t",
        escaped_char,
        "\\t",
        EscapedChar,
        "\t"
    ),
    token_alt_text!(
        "escaped_char",
        "alpha_token",
        "alpha:r",
        escaped_char,
        "\\r",
        EscapedChar,
        "\r"
    ),
    token_alt_text!(
        "escaped_char",
        "alpha_token",
        "alpha:x",
        escaped_char,
        "\\x",
        EscapedChar,
        "x"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "ampersand",
        escaped_char,
        "\\&",
        EscapedChar,
        "&"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "grave",
        escaped_char,
        "\\`",
        EscapedChar,
        "`"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "dollar",
        escaped_char,
        "\\$",
        EscapedChar,
        "$"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "bar",
        escaped_char,
        "\\|",
        EscapedChar,
        "|"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "percent",
        escaped_char,
        "\\%",
        EscapedChar,
        "%"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "at",
        escaped_char,
        "\\@",
        EscapedChar,
        "@"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "slash",
        escaped_char,
        "\\/",
        EscapedChar,
        "/"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "hashtag",
        escaped_char,
        "\\#",
        EscapedChar,
        "#"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "equal",
        escaped_char,
        "\\=",
        EscapedChar,
        "="
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "backslash",
        escaped_char,
        "\\\\",
        EscapedChar,
        "\\"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "tilde",
        escaped_char,
        "\\~",
        EscapedChar,
        "~"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "plus",
        escaped_char,
        "\\+",
        EscapedChar,
        "+"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "dash",
        escaped_char,
        "\\-",
        EscapedChar,
        "-"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "asterisk",
        escaped_char,
        "\\*",
        EscapedChar,
        "*"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "caret",
        escaped_char,
        "\\^",
        EscapedChar,
        "^"
    ),
    token_alt_text!(
        "escaped_char",
        "symbol",
        "underscore",
        escaped_char,
        "\\_",
        EscapedChar,
        "_"
    ),
    token_alt_text!(
        "escaped_char",
        "punctuation",
        "period",
        escaped_char,
        "\\.",
        EscapedChar,
        "."
    ),
    token_alt_text!(
        "escaped_char",
        "punctuation",
        "exclamation",
        escaped_char,
        "\\!",
        EscapedChar,
        "!"
    ),
    token_alt_text!(
        "escaped_char",
        "punctuation",
        "question",
        escaped_char,
        "\\?",
        EscapedChar,
        "?"
    ),
    token_alt_text!(
        "escaped_char",
        "punctuation",
        "comma",
        escaped_char,
        "\\,",
        EscapedChar,
        ","
    ),
    token_alt_text!(
        "escaped_char",
        "punctuation",
        "colon",
        escaped_char,
        "\\:",
        EscapedChar,
        ":"
    ),
    token_alt_text!(
        "escaped_char",
        "punctuation",
        "semicolon",
        escaped_char,
        "\\;",
        EscapedChar,
        ";"
    ),
    token_alt_text!(
        "escaped_char",
        "punctuation",
        "quote",
        escaped_char,
        "\\\"",
        EscapedChar,
        "\""
    ),
    token_alt_text!(
        "escaped_char",
        "punctuation",
        "apostrophe",
        escaped_char,
        "\\'",
        EscapedChar,
        "'"
    ),
    token_alt!("text", "alpha_token", "alpha:A", text, "A", Alpha),
    token_alt!("text", "digit_token", "digit:7", text, "7", Digit),
    token_alt!("text", "emoji", "emoji:robot", text, "🤖", Emoji),
    token_alt!(
        "text",
        "forbidden_emoji",
        "box_vert_bold",
        text,
        "┃",
        BoxDrawing
    ),
    token_alt!("text", "space", "space", text, " ", Space),
    token_alt!("text", "tab", "tab", text, "\t", Tab),
    token_alt_text!(
        "text",
        "escaped_char",
        "alpha:n",
        text,
        "\\n",
        EscapedChar,
        "\n"
    ),
    token_alt!("text", "punctuation", "question", text, "?", Question),
    token_alt!(
        "text",
        "grouping_symbol",
        "left_angle2",
        text,
        "⟨",
        LeftAngle
    ),
    token_alt!("text", "symbol", "percent", text, "%", Percent),
    token_alt!("raw_text", "alpha_token", "alpha:A", raw_text, "A", Alpha),
    token_alt!("raw_text", "digit_token", "digit:7", raw_text, "7", Digit),
    token_alt!("raw_text", "emoji", "emoji:robot", raw_text, "🤖", Emoji),
    token_alt!(
        "raw_text",
        "forbidden_emoji",
        "box_vert_bold",
        raw_text,
        "┃",
        BoxDrawing
    ),
    token_alt!("raw_text", "space", "space", raw_text, " ", Space),
    token_alt!("raw_text", "tab", "tab", raw_text, "\t", Tab),
    token_alt!(
        "raw_text",
        "punctuation",
        "question",
        raw_text,
        "?",
        Question
    ),
    token_alt!(
        "raw_text",
        "grouping_symbol",
        "left_angle2",
        raw_text,
        "⟨",
        LeftAngle
    ),
    token_alt!("raw_text", "symbol", "percent", raw_text, "%", Percent),
    token_alt!(
        "matrix_start",
        "box_tl_round",
        "box_tl_round",
        matrix_start,
        "╭",
        BoxDrawing
    ),
    token_alt!(
        "matrix_start",
        "box_tl",
        "box_tl",
        matrix_start,
        "┌",
        BoxDrawing
    ),
    token_alt!(
        "matrix_start",
        "box_tl_bold",
        "box_tl_bold",
        matrix_start,
        "┏",
        BoxDrawing
    ),
    token_alt!(
        "matrix_start",
        "left_bracket",
        "left_bracket",
        matrix_start,
        "[",
        LeftBracket
    ),
    token_alt!(
        "matrix_end",
        "box_br_round",
        "box_br_round",
        matrix_end,
        "╯",
        BoxDrawing
    ),
    token_alt!(
        "matrix_end",
        "box_br",
        "box_br",
        matrix_end,
        "┘",
        BoxDrawing
    ),
    token_alt!(
        "matrix_end",
        "box_br_bold",
        "box_br_bold",
        matrix_end,
        "┛",
        BoxDrawing
    ),
    token_alt!(
        "matrix_end",
        "right_bracket",
        "right_bracket",
        matrix_end,
        "]",
        RightBracket
    ),
    token_alt!(
        "box_drawing_char",
        "box_tl",
        "box_tl",
        box_drawing_char,
        "┌",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_bl",
        "box_bl",
        box_drawing_char,
        "└",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_tr",
        "box_tr",
        box_drawing_char,
        "┐",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_tl_bold",
        "box_tl_bold",
        box_drawing_char,
        "┏",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_bl_bold",
        "box_bl_bold",
        box_drawing_char,
        "┗",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_tr_bold",
        "box_tr_bold",
        box_drawing_char,
        "┓",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_tr_round",
        "box_tr_round",
        box_drawing_char,
        "╮",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_bl_round",
        "box_bl_round",
        box_drawing_char,
        "╰",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_vert",
        "box_vert",
        box_drawing_char,
        "│",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_cross",
        "box_cross",
        box_drawing_char,
        "┼",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_horz",
        "box_horz",
        box_drawing_char,
        "─",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_t_left",
        "box_t_left",
        box_drawing_char,
        "├",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_t_right",
        "box_t_right",
        box_drawing_char,
        "┤",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_t_top",
        "box_t_top",
        box_drawing_char,
        "┬",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_char",
        "box_t_bottom",
        "box_t_bottom",
        box_drawing_char,
        "┴",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_vert_bold",
        "box_vert_bold",
        box_drawing_emoji,
        "┃",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_tl",
        "box_tl",
        box_drawing_emoji,
        "┌",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_bl",
        "box_bl",
        box_drawing_emoji,
        "└",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_tr",
        "box_tr",
        box_drawing_emoji,
        "┐",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_tl_bold",
        "box_tl_bold",
        box_drawing_emoji,
        "┏",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_bl_bold",
        "box_bl_bold",
        box_drawing_emoji,
        "┗",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_tr_bold",
        "box_tr_bold",
        box_drawing_emoji,
        "┓",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_tl_round",
        "box_tl_round",
        box_drawing_emoji,
        "╭",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_br_round",
        "box_br_round",
        box_drawing_emoji,
        "╯",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_tr_round",
        "box_tr_round",
        box_drawing_emoji,
        "╮",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_bl_round",
        "box_bl_round",
        box_drawing_emoji,
        "╰",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_vert",
        "box_vert",
        box_drawing_emoji,
        "│",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_cross",
        "box_cross",
        box_drawing_emoji,
        "┼",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_horz",
        "box_horz",
        box_drawing_emoji,
        "─",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_t_left",
        "box_t_left",
        box_drawing_emoji,
        "├",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_t_right",
        "box_t_right",
        box_drawing_emoji,
        "┤",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_t_top",
        "box_t_top",
        box_drawing_emoji,
        "┬",
        BoxDrawing
    ),
    token_alt!(
        "box_drawing_emoji",
        "box_t_bottom",
        "box_t_bottom",
        box_drawing_emoji,
        "┴",
        BoxDrawing
    ),
    token_alt!(
        "table_start",
        "box_tl_round",
        "box_tl_round",
        table_start,
        "╭",
        BoxDrawing
    ),
    token_alt!(
        "table_start",
        "box_tl",
        "box_tl",
        table_start,
        "┌",
        BoxDrawing
    ),
    token_alt!(
        "table_start",
        "box_tl_bold",
        "box_tl_bold",
        table_start,
        "┏",
        BoxDrawing
    ),
    token_alt!(
        "table_start",
        "left_brace",
        "left_brace",
        table_start,
        "{",
        LeftBrace
    ),
    token_alt!(
        "table_start",
        "table_separator",
        "box_vert",
        table_start,
        "│",
        BoxDrawing
    ),
    token_alt!(
        "table_start",
        "table_separator",
        "box_vert_bold",
        table_start,
        "┃",
        BoxDrawing
    ),
    token_alt!(
        "table_start",
        "table_separator",
        "bar",
        table_start,
        "|",
        Bar
    ),
    token_alt!(
        "table_end",
        "box_br_round",
        "box_br_round",
        table_end,
        "╯",
        BoxDrawing
    ),
    token_alt!("table_end", "box_br", "box_br", table_end, "┘", BoxDrawing),
    token_alt!(
        "table_end",
        "box_br_bold",
        "box_br_bold",
        table_end,
        "┛",
        BoxDrawing
    ),
    token_alt!(
        "table_end",
        "right_brace",
        "right_brace",
        table_end,
        "}",
        RightBrace
    ),
    token_alt!(
        "table_end",
        "table_separator",
        "box_vert",
        table_end,
        "│",
        BoxDrawing
    ),
    token_alt!(
        "table_end",
        "table_separator",
        "box_vert_bold",
        table_end,
        "┃",
        BoxDrawing
    ),
    token_alt!("table_end", "table_separator", "bar", table_end, "|", Bar),
    token_alt_ws!(
        "table_separator",
        "box_vert",
        "box_vert",
        table_separator,
        "│",
        BoxDrawing
    ),
    token_alt_ws!(
        "table_separator",
        "box_vert_bold",
        "box_vert_bold",
        table_separator,
        "┃",
        BoxDrawing
    ),
    token_alt_ws!("table_separator", "bar", "bar", table_separator, "|", Bar),
    token_alt!("table_horz", "dash", "dash", table_horz, "-", Dash),
    token_alt!(
        "table_horz",
        "box_horz",
        "box_horz",
        table_horz,
        "─",
        BoxDrawing
    ),
];

type UnitParser = for<'source> fn(ParseString<'source>) -> ParseResult<'source, ()>;

#[derive(Clone, Copy)]
struct UnitAlternativeContract {
    wrapper: &'static str,
    direct_branch: &'static str,
    parser: UnitParser,
    literal: &'static str,
    surrounded_whitespace: bool,
}

const UNIT_ALTERNATIVE_CONTRACTS: &[UnitAlternativeContract] = &[
    UnitAlternativeContract {
        wrapper: "guard_operator",
        direct_branch: "tag:|",
        parser: guard_operator,
        literal: "|",
        surrounded_whitespace: true,
    },
    UnitAlternativeContract {
        wrapper: "guard_operator",
        direct_branch: "tag:│",
        parser: guard_operator,
        literal: "│",
        surrounded_whitespace: true,
    },
    UnitAlternativeContract {
        wrapper: "guard_operator",
        direct_branch: "tag:├",
        parser: guard_operator,
        literal: "├",
        surrounded_whitespace: true,
    },
    UnitAlternativeContract {
        wrapper: "guard_operator",
        direct_branch: "tag:└",
        parser: guard_operator,
        literal: "└",
        surrounded_whitespace: true,
    },
    UnitAlternativeContract {
        wrapper: "space_tab1",
        direct_branch: "many1:space_tab",
        parser: space_tab1,
        literal: " \t\u{00A0}\u{2009}",
        surrounded_whitespace: false,
    },
];

const SPREAD_ALTERNATIVE_CONTRACTS: &[(&str, &str, &str)] = &[
    ("spread_operator_a", "...", "[head, ..., tail]"),
    ("spread_operator_u", "…", "[head, …, tail]"),
];

const IDENTIFIER_ALTERNATIVE_CONTRACTS: &[(&str, &str)] = &[
    ("alpha-start-with-every-symbol-rest", "alpha&$%/#\\~+-*^9🤖"),
    ("emoji-start", "🤖a7"),
];

const CODEBLOCK_SIGIL_CONTRACTS: &[(&str, &str, TerminalParser)] = &[
    (
        "grave_codeblock_sigil",
        "```",
        grave_codeblock_sigil as TerminalParser,
    ),
    (
        "tilde_codeblock_sigil",
        "~~~",
        tilde_codeblock_sigil as TerminalParser,
    ),
];

const FLOAT_SIGIL_CONTRACTS: &[(&str, &str, FloatDirection)] = &[
    ("float_left", "<<:", FloatDirection::Left),
    ("float_right", ":>>", FloatDirection::Right),
];

const OP_ASSIGN_CONTRACTS: &[(&str, &str, OpAssignOp)] = &[
    ("add_assign_operator", "+=", OpAssignOp::Add),
    ("sub_assign_operator", "-=", OpAssignOp::Sub),
    ("mul_assign_operator", "*=", OpAssignOp::Mul),
    ("div_assign_operator", "/=", OpAssignOp::Div),
    ("exp_assign_operator", "^=", OpAssignOp::Exp),
];

const GENERATOR_ARROW_CONTRACTS: &[(&str, &str, &str)] = &[
    ("generator_arrow", "<-", "x <- xs"),
    ("generator_arrow_u", "←", "x ← xs"),
];

const CHECKED_MARKER_CONTRACTS: &[(&str, &str, bool)] = &[
    ("tag:x", "-[x]done\n", true),
    ("tag:✓", "-[✓]done\n", true),
    ("tag:✗", "-[✗]done\n", true),
    ("whitespace0:empty", "-[]done\n", false),
    ("whitespace0:space", "-[ ]done\n", false),
    ("whitespace0:tab", "-[\t]done\n", false),
];

const SUCCESS_ERROR_SIGIL_CONTRACTS: &[(&str, &str, bool)] = &[
    ("success_sigil", "(+)> ok\n", true),
    ("success_check_sigil", "(✓)> ok\n", true),
    ("error_sigil", "(x)> error\n", false),
    ("error_alt_sigil", "(✗)> error\n", false),
];

#[cfg(feature = "mika")]
const MIKA_ARM_LEFT_CONTRACTS: &[(&str, MikaArm)] = &[
    ("Ɔ∞", MikaArm::BigGripperLeft),
    ("›─", MikaArm::GripperLeft),
    ("›⌣", MikaArm::GestureLeft),
    ("·¬", MikaArm::ShootLeft),
    ("-◡", MikaArm::ShrugLeft),
    ("ᗑ", MikaArm::BatWing),
    ("ᕦ", MikaArm::CurlLeft),
    ("~", MikaArm::Dance),
    ("⌣", MikaArm::GestureLeft),
    ("╭", MikaArm::Left),
    ("⸌", MikaArm::RaisedLeft),
    ("⸸", MikaArm::Sword),
    ("─", MikaArm::Point),
    ("ᓂ", MikaArm::PunchLeft),
    ("ᓇ", MikaArm::PunchLowLeft),
    ("╰", MikaArm::UpLeft),
];

#[cfg(feature = "mika")]
const MIKA_ARM_RIGHT_CONTRACTS: &[(&str, MikaArm)] = &[
    ("∞C", MikaArm::BigGripperRight),
    ("─‹", MikaArm::GripperRight),
    ("⌣‹", MikaArm::GestureRight),
    ("⌐·", MikaArm::ShootRight),
    ("◡-", MikaArm::ShrugRight),
    ("ᗑ", MikaArm::BatWing),
    ("ᕤ", MikaArm::CurlRight),
    ("~", MikaArm::Dance),
    ("⌣", MikaArm::GestureRight),
    ("╮", MikaArm::Right),
    ("⸍", MikaArm::RaisedRight),
    ("ᗢ", MikaArm::Shield),
    ("─", MikaArm::Point),
    ("ᓀ", MikaArm::PunchRight),
    ("ᓄ", MikaArm::PunchLowRight),
    ("╯", MikaArm::UpRight),
];

#[cfg(feature = "mika")]
macro_rules! mika_eye_variants {
    ($eye:ident) => {
        &[
            $eye::Content,
            $eye::Confused,
            $eye::Crying,
            $eye::Dazed,
            $eye::Dead,
            $eye::EyesSqueezed,
            $eye::SuperSqueezed,
            $eye::Glaring,
            $eye::Happy,
            $eye::Normal,
            $eye::PeerRight,
            $eye::PeerStraight,
            $eye::Pleased,
            $eye::Resolved,
            $eye::RollingEyes,
            $eye::Sad,
            $eye::Scared,
            $eye::Shades,
            $eye::Sleeping,
            $eye::Smiling,
            $eye::Squinting,
            $eye::Surprised,
            $eye::TearingUp,
            $eye::Unimpressed,
            $eye::Wired,
        ]
    };
}

#[cfg(feature = "mika")]
const MIKA_EYE_LEFT_CONTRACTS: &[MikaEyeLeft] = mika_eye_variants!(MikaEyeLeft);
#[cfg(feature = "mika")]
const MIKA_EYE_RIGHT_CONTRACTS: &[MikaEyeRight] = mika_eye_variants!(MikaEyeRight);

#[cfg(feature = "mika")]
const MIKA_NOSE_CONTRACTS: &[(MikaNose, MikaNose)] = &[
    (MikaNose::Normal, MikaNose::Normal),
    (MikaNose::Open, MikaNose::Open),
    (MikaNose::Back, MikaNose::Back),
    (MikaNose::Stage1, MikaNose::Stage1),
    (MikaNose::Stage2, MikaNose::Stage2),
    // Stage2 and Stage3 expose the same glyph; ordered matching selects Stage2.
    (MikaNose::Stage3, MikaNose::Stage2),
    (MikaNose::Blink, MikaNose::Blink),
    (MikaNose::Wide, MikaNose::Wide),
    (MikaNose::Error, MikaNose::Error),
    (MikaNose::Filled, MikaNose::Filled),
    (MikaNose::FlatMouth, MikaNose::FlatMouth),
    (MikaNose::Hexagon, MikaNose::Hexagon),
    (MikaNose::Pentagon, MikaNose::Pentagon),
    (MikaNose::Hexagon2, MikaNose::Hexagon2),
    (MikaNose::HexagonOpen, MikaNose::HexagonOpen),
];

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/grammar")
}

fn relative_files(root: &Path, directories: &[&str]) -> BTreeSet<String> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeSet<String>) {
        for entry in fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
        {
            let path = entry
                .expect("fixture directory entry must be readable")
                .path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("fixture must be below its root")
                    .to_string_lossy()
                    .replace('\\', "/");
                assert!(files.insert(relative), "duplicate fixture path");
            }
        }
    }

    let mut files = BTreeSet::new();
    for directory in directories {
        visit(root, &root.join(directory), &mut files);
    }
    files
}

fn read_source(path: &Path) -> String {
    let mut source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    // Fixture files are normal POSIX text files. Their one terminating LF is
    // storage syntax rather than test input. An intentional final source LF is
    // represented by one additional blank line in the fixture.
    if source.ends_with('\n') {
        source.pop();
        if source.ends_with('\r') {
            source.pop();
        }
    }
    source
}

fn read_cases() -> Vec<Case> {
    let path = fixture_root().join("cases.tsv");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let mut lines = text.lines();
    let header = lines.next().expect("cases.tsv must have a header");
    assert_eq!(
        header.split('\t').collect::<Vec<_>>(),
        HEADER,
        "unexpected cases.tsv columns"
    );

    lines
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|(index, line)| {
            let columns = line.split('\t').collect::<Vec<_>>();
            assert_eq!(
                columns.len(),
                HEADER.len(),
                "cases.tsv line {} has {} columns, expected {}",
                index + 2,
                columns.len(),
                HEADER.len()
            );
            Case {
                id: columns[0].to_owned(),
                rule: columns[1].to_owned(),
                entry_point: columns[2].to_owned(),
                feature_set: columns[3].to_owned(),
                expected_result: columns[4].to_owned(),
                source_file: columns[5].to_owned(),
                snapshot_file: columns[6].to_owned(),
                notes: columns[7].to_owned(),
            }
        })
        .collect()
}

fn enabled_for_this_build(feature_set: &str) -> bool {
    match feature_set {
        "all" => true,
        // `base` is an umbrella feature, while the default feature list spells
        // out its members.  This makes the two required test commands
        // distinguishable without introducing a production cfg.
        "default" => !cfg!(feature = "base"),
        "base" => cfg!(feature = "base"),
        "invariant-define" => cfg!(feature = "invariant_define"),
        "mika" => cfg!(feature = "mika"),
        "mika-disabled" => !cfg!(feature = "mika"),
        other => panic!("unknown feature-set {other:?}"),
    }
}

fn normalize(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(normalize),
        Value::Object(object) => {
            let is_source_location = |value: &Value| {
                matches!(
                    value,
                    Value::Object(location)
                        if location.len() == 2
                            && location.contains_key("row")
                            && location.contains_key("col")
                )
            };
            let is_positional_source_range = object.len() == 2
                && object.get("start").is_some_and(is_source_location)
                && object.get("end").is_some_and(is_source_location);
            if is_positional_source_range {
                // Tuple enum fields (for example `MechCode::Error`) do not
                // carry a serde field name, so recognize SourceRange by shape.
                *value = Value::Null;
                return;
            }

            // Ranges and diagnostic annotations are intentionally unstable.
            // In particular, never snapshot ParseErrorDetail.message.
            for key in [
                "src_range",
                "error_range",
                "cause_range",
                "cause_rng",
                "annotation_rngs",
                "annotation_ranges",
            ] {
                object.remove(key);
            }
            object.values_mut().for_each(normalize);

            // serde_json does not promise insertion ordering without its
            // preserve_order feature.  Sort explicitly for reviewable files.
            let old = std::mem::take(object);
            let mut sorted = BTreeMap::new();
            sorted.extend(old);
            object.extend(sorted);
        }
        _ => {}
    }
}

fn serialize<T: Serialize>(output: &T) -> Value {
    let mut value = serde_json::to_value(output).expect("AST must serialize");
    normalize(&mut value);
    value
}

fn run_nom<T>(source: &str, parser: fn(ParseString<'_>) -> ParseResult<'_, T>) -> ParseOutcome
where
    T: Serialize,
{
    let graphemes = graphemes::init_tag(source);
    let source_len = graphemes.len();
    match parser(ParseString::new(&graphemes)) {
        Ok((remaining, output)) => {
            let consumed = remaining.cursor.min(source_len);
            let unconsumed = graphemes[consumed..source_len].join("");
            ParseOutcome::Success {
                ast: serialize(&output),
                consumed,
                source_len,
                remaining: unconsumed,
                diagnostics: remaining.error_log.len(),
            }
        }
        Err(_) => ParseOutcome::Failure,
    }
}

fn run_normalized_nom<T>(
    source: &str,
    parser: fn(ParseString<'_>) -> ParseResult<'_, T>,
) -> ParseOutcome
where
    T: Serialize,
{
    let graphemes = graphemes::init_source(source);
    let source_len = graphemes.len();
    match parser(ParseString::new(&graphemes)) {
        Ok((remaining, output)) => {
            let consumed = remaining.cursor;
            let unconsumed = graphemes[consumed..source_len].join("");
            ParseOutcome::Success {
                ast: serialize(&output),
                consumed,
                source_len,
                remaining: unconsumed,
                diagnostics: remaining.error_log.len(),
            }
        }
        Err(_) => ParseOutcome::Failure,
    }
}

fn run_public_parse(source: &str) -> ParseOutcome {
    match parse(source) {
        Ok(program) => ParseOutcome::Success {
            ast: serialize(&program),
            consumed: source.graphemes(true).count(),
            source_len: source.graphemes(true).count(),
            remaining: String::new(),
            diagnostics: 0,
        },
        Err(_) => ParseOutcome::Failure,
    }
}

fn run_public_grammar(source: &str) -> ParseOutcome {
    match parse_grammar(source) {
        Ok(grammar) => ParseOutcome::Success {
            ast: serialize(&grammar),
            consumed: source.graphemes(true).count(),
            source_len: source.graphemes(true).count(),
            remaining: String::new(),
            diagnostics: 0,
        },
        Err(_) => ParseOutcome::Failure,
    }
}

fn assert_full_terminal_parse(
    contract: &FixedTerminalContract,
    source: &str,
    probe: &str,
) -> Token {
    let graphemes = graphemes::init_tag(source);
    let source_len = graphemes.len();
    let (remaining, token) =
        (contract.parser)(ParseString::new(&graphemes)).unwrap_or_else(|error| {
            panic!(
                "{} ({}) rejected its {probe} {:?}: {error:?}",
                contract.production_id, contract.function_name, source
            )
        });
    assert_eq!(
        remaining.cursor, source_len,
        "{} ({}) accepted only a prefix of its {probe} {:?}",
        contract.production_id, contract.function_name, source
    );
    assert!(
        remaining.error_log.is_empty(),
        "{} ({}) logged a diagnostic for its {probe} {:?}",
        contract.production_id,
        contract.function_name,
        source
    );
    assert_eq!(
        token.kind, contract.expected_kind,
        "{} ({}) returned the wrong TokenKind for its {probe}",
        contract.production_id, contract.function_name
    );
    assert_eq!(
        token.chars,
        contract.literal.chars().collect::<Vec<_>>(),
        "{} ({}) returned the wrong token text for its {probe}",
        contract.production_id,
        contract.function_name
    );
    token
}

fn run_terminal_contracts(source: &str) -> ParseOutcome {
    const MARKER: &str = "fixed-terminal-contracts-v1";
    if source != MARKER {
        return ParseOutcome::Failure;
    }

    let ast = FIXED_TERMINAL_CONTRACTS
        .iter()
        .map(|contract| {
            let token = assert_full_terminal_parse(contract, contract.literal, "declared literal");
            let text = token.chars.iter().collect::<String>();
            let mut row = json!({
                "function": contract.function_name,
                "inventory_id": contract.production_id,
                "parser_macro": contract.parser_macro,
                "text": text,
                "token": serialize(&token),
            });
            if contract.parser_macro == "ws0_leaf" {
                let surrounded_source = format!(" {} ", contract.literal);
                let surrounded_token = assert_full_terminal_parse(
                    contract,
                    &surrounded_source,
                    "surrounded-whitespace literal",
                );
                row.as_object_mut()
                    .expect("terminal snapshot row must be an object")
                    .insert(
                        "surrounded_whitespace".to_owned(),
                        json!({
                            "source": surrounded_source,
                            "text": surrounded_token.chars.iter().collect::<String>(),
                            "token": serialize(&surrounded_token),
                        }),
                    );
            }
            row
        })
        .collect::<Vec<_>>();

    let leaf = FIXED_TERMINAL_CONTRACTS
        .iter()
        .find(|contract| contract.production_id == "base.ampersand")
        .expect("representative leaf contract must exist");
    assert_eq!(
        leaf.parser_macro, "leaf",
        "representative terminal must use leaf!"
    );
    let trailing_source = format!("{} ", leaf.literal);
    let trailing_graphemes = graphemes::init_tag(&trailing_source);
    let (remaining, token) = (leaf.parser)(ParseString::new(&trailing_graphemes))
        .expect("leaf! must accept its literal before trailing whitespace");
    let literal_len = leaf.literal.graphemes(true).count();
    assert_eq!(
        remaining.cursor, literal_len,
        "leaf! unexpectedly consumed trailing whitespace"
    );
    assert_eq!(
        trailing_graphemes[remaining.cursor..].join(""),
        " ",
        "leaf! must leave trailing whitespace for its caller"
    );
    assert_eq!(token.kind, leaf.expected_kind);
    assert_eq!(token.chars, leaf.literal.chars().collect::<Vec<_>>());

    let leading_source = format!(" {}", leaf.literal);
    let leading_graphemes = graphemes::init_tag(&leading_source);
    assert!(
        (leaf.parser)(ParseString::new(&leading_graphemes)).is_err(),
        "leaf! must not consume leading whitespace"
    );

    let source_len = source.graphemes(true).count();
    ParseOutcome::Success {
        ast: Value::Array(ast),
        consumed: source_len,
        source_len,
        remaining: String::new(),
        diagnostics: 0,
    }
}

fn assert_complete_value<T: Serialize>(
    source: &str,
    parser: for<'parser> fn(ParseString<'parser>) -> ParseResult<'parser, T>,
    label: &str,
) -> Value {
    let graphemes = graphemes::init_tag(source);
    let source_len = graphemes.len();
    let (remaining, output) = parser(ParseString::new(&graphemes))
        .unwrap_or_else(|error| panic!("{label} rejected {source:?}: {error:?}"));
    assert_eq!(
        remaining.cursor, source_len,
        "{label} accepted only a prefix of {source:?}"
    );
    assert!(
        remaining.error_log.is_empty(),
        "{label} logged a diagnostic for {source:?}"
    );
    serialize(&output)
}

fn assert_complete_token(source: &str, parser: TerminalParser, label: &str) -> Token {
    let graphemes = graphemes::init_tag(source);
    let source_len = graphemes.len();
    let (remaining, token) = parser(ParseString::new(&graphemes))
        .unwrap_or_else(|error| panic!("{label} rejected {source:?}: {error:?}"));
    assert_eq!(
        remaining.cursor, source_len,
        "{label} accepted only a prefix of {source:?}"
    );
    assert!(
        remaining.error_log.is_empty(),
        "{label} logged a diagnostic for {source:?}"
    );
    token
}

fn run_alternative_terminal_contracts(source: &str) -> ParseOutcome {
    const MARKER: &str = "alternative-terminal-contracts-v1";
    if source != MARKER {
        return ParseOutcome::Failure;
    }

    let token_alternatives = ALTERNATIVE_TOKEN_CONTRACTS
        .iter()
        .map(|contract| {
            let input = if contract.surrounded_whitespace {
                format!(" {} ", contract.literal)
            } else {
                contract.literal.to_owned()
            };
            let token = assert_complete_token(&input, contract.parser, contract.wrapper);
            assert_eq!(
                token.kind, contract.expected_kind,
                "{}:{} returned the wrong TokenKind",
                contract.wrapper, contract.leaf_branch
            );
            assert_eq!(
                token.chars,
                contract.expected_text.chars().collect::<Vec<_>>(),
                "{}:{} returned the wrong token text",
                contract.wrapper,
                contract.leaf_branch
            );
            let value = serialize(&token);
            json!({
                "direct_branch": contract.direct_branch,
                "input": input,
                "leaf_branch": contract.leaf_branch,
                "literal": contract.literal,
                "token": value,
                "wrapper": contract.wrapper,
            })
        })
        .collect::<Vec<_>>();

    let unit_alternatives = UNIT_ALTERNATIVE_CONTRACTS
        .iter()
        .map(|contract| {
            let input = if contract.surrounded_whitespace {
                format!(" {} ", contract.literal)
            } else {
                contract.literal.to_owned()
            };
            let value = assert_complete_value(&input, contract.parser, contract.wrapper);
            assert_eq!(value, Value::Null, "{} must return unit", contract.wrapper);
            json!({
                "direct_branch": contract.direct_branch,
                "input": input,
                "output": value,
                "wrapper": contract.wrapper,
            })
        })
        .collect::<Vec<_>>();
    let empty = graphemes::init_tag("");
    assert!(
        space_tab1(ParseString::new(&empty)).is_err(),
        "space_tab1 must reject empty input"
    );

    let spread_alternatives = SPREAD_ALTERNATIVE_CONTRACTS
        .iter()
        .map(|(branch, literal, input)| {
            json!({
                "ast": assert_complete_value(input, pattern_array, "spread_operator via pattern_array"),
                "direct_branch": branch,
                "literal": literal,
            })
        })
        .collect::<Vec<_>>();

    let identifier_alternatives = IDENTIFIER_ALTERNATIVE_CONTRACTS
        .iter()
        .map(|(branch, input)| {
            json!({
                "ast": assert_complete_value(input, identifier, "identifier"),
                "branch": branch,
                "input": input,
            })
        })
        .collect::<Vec<_>>();

    let codeblock_sigils = CODEBLOCK_SIGIL_CONTRACTS
        .iter()
        .copied()
        .map(|(branch, literal, expected_parser)| {
            let graphemes = graphemes::init_tag(literal);
            let (remaining, selected_parser) = codeblock_sigil(ParseString::new(&graphemes))
                .unwrap_or_else(|error| panic!("codeblock_sigil rejected {literal:?}: {error:?}"));
            assert_eq!(remaining.cursor, graphemes.len());
            assert!(remaining.error_log.is_empty());
            assert!(
                std::ptr::fn_addr_eq(selected_parser, expected_parser),
                "codeblock_sigil selected the wrong closing parser for {literal:?}"
            );
            json!({
                "branch": branch,
                "literal": literal,
                "selected_closing_parser": branch,
            })
        })
        .collect::<Vec<_>>();

    let float_sigils = FLOAT_SIGIL_CONTRACTS
        .iter()
        .map(|(branch, literal, expected)| {
            let value = assert_complete_value(literal, float_sigil, "float_sigil");
            assert_eq!(value, serialize(&expected));
            json!({"branch": branch, "literal": literal, "output": value})
        })
        .collect::<Vec<_>>();

    let op_assign_operators = OP_ASSIGN_CONTRACTS
        .iter()
        .map(|(branch, literal, expected)| {
            let input = format!(" {literal} ");
            let value = assert_complete_value(&input, op_assign_operator, "op_assign_operator");
            assert_eq!(value, serialize(&expected));
            json!({"branch": branch, "input": input, "literal": literal, "output": value})
        })
        .collect::<Vec<_>>();

    let generator_arrows = GENERATOR_ARROW_CONTRACTS
        .iter()
        .copied()
        .map(|(branch, literal, input)| {
            json!({
                "ast": assert_complete_value(input, generator, "generator"),
                "branch": branch,
                "literal": literal,
            })
        })
        .collect::<Vec<_>>();

    let checked_markers = CHECKED_MARKER_CONTRACTS
        .iter()
        .copied()
        .map(|(branch, input, checked)| {
            let value = if checked {
                assert_complete_value(input, checked_item, "checked_item")
            } else {
                assert_complete_value(input, unchecked_item, "unchecked_item")
            };
            assert_eq!(
                value.as_array().and_then(|tuple| tuple.first()),
                Some(&Value::Bool(checked)),
                "{branch} returned the wrong checked state"
            );
            json!({"branch": branch, "input": input, "output": value})
        })
        .collect::<Vec<_>>();

    let success_error_sigils = SUCCESS_ERROR_SIGIL_CONTRACTS
        .iter()
        .copied()
        .map(|(branch, input, success)| {
            let value = if success {
                assert_complete_value(input, success_block, "success_block")
            } else {
                assert_complete_value(input, error_block, "error_block")
            };
            json!({"branch": branch, "input": input, "output": value})
        })
        .collect::<Vec<_>>();

    let ast = json!({
        "checked_markers": checked_markers,
        "codeblock_sigils": codeblock_sigils,
        "float_sigils": float_sigils,
        "generator_arrows": generator_arrows,
        "identifier_alternatives": identifier_alternatives,
        "op_assign_operators": op_assign_operators,
        "spread_alternatives": spread_alternatives,
        "success_error_sigils": success_error_sigils,
        "token_alternatives": token_alternatives,
        "unit_alternatives": unit_alternatives,
    });
    let source_len = source.graphemes(true).count();
    ParseOutcome::Success {
        ast,
        consumed: source_len,
        source_len,
        remaining: String::new(),
        diagnostics: 0,
    }
}

#[cfg(feature = "mika")]
fn assert_complete_expected<T>(
    source: &str,
    parser: for<'parser> fn(ParseString<'parser>) -> ParseResult<'parser, T>,
    expected: &T,
    label: &str,
) -> Value
where
    T: Serialize + std::fmt::Debug + PartialEq,
{
    let graphemes = graphemes::init_tag(source);
    let (remaining, output) = parser(ParseString::new(&graphemes))
        .unwrap_or_else(|error| panic!("{label} rejected {source:?}: {error:?}"));
    assert_eq!(
        remaining.cursor,
        graphemes.len(),
        "{label} accepted only a prefix of {source:?}"
    );
    assert!(
        remaining.error_log.is_empty(),
        "{label} logged a diagnostic for {source:?}"
    );
    assert_eq!(
        &output, expected,
        "{label} returned the wrong enum variant for {source:?}"
    );
    serialize(&output)
}

#[cfg(feature = "mika")]
fn run_mika_alternative_terminal_contracts(source: &str) -> ParseOutcome {
    const MARKER: &str = "mika-alternative-terminal-contracts-v1";
    if source != MARKER {
        return ParseOutcome::Failure;
    }

    let mut arms = Vec::new();
    for &(literal, ref expected) in MIKA_ARM_LEFT_CONTRACTS {
        arms.push(json!({
            "literal": literal,
            "output": assert_complete_expected(literal, mika_arm_left, expected, "mika_arm_left"),
            "side": "left",
        }));
    }
    for &(literal, ref expected) in MIKA_ARM_RIGHT_CONTRACTS {
        arms.push(json!({
            "literal": literal,
            "output": assert_complete_expected(literal, mika_arm_right, expected, "mika_arm_right"),
            "side": "right",
        }));
    }

    let left_eyes = MIKA_EYE_LEFT_CONTRACTS
        .iter()
        .map(|expected| {
            let literal = expected.symbol();
            json!({
                "literal": literal,
                "output": assert_complete_expected(literal, mika_eye_left, expected, "mika_eye_left"),
            })
        })
        .collect::<Vec<_>>();
    let right_eyes = MIKA_EYE_RIGHT_CONTRACTS
        .iter()
        .map(|expected| {
            let literal = expected.symbol();
            json!({
                "literal": literal,
                "output": assert_complete_expected(literal, mika_eye_right, expected, "mika_eye_right"),
            })
        })
        .collect::<Vec<_>>();
    let noses = MIKA_NOSE_CONTRACTS
        .iter()
        .map(|(declared, expected)| {
            let literal = declared.symbol();
            json!({
                "declared_variant": serialize(declared),
                "literal": literal,
                "output": assert_complete_expected(literal, mika_nose, expected, "mika_nose"),
            })
        })
        .collect::<Vec<_>>();

    let ast = json!({
        "arms": arms,
        "left_eyes": left_eyes,
        "noses": noses,
        "right_eyes": right_eyes,
    });
    let source_len = source.graphemes(true).count();
    ParseOutcome::Success {
        ast,
        consumed: source_len,
        source_len,
        remaining: String::new(),
        diagnostics: 0,
    }
}

#[cfg(not(feature = "mika"))]
fn run_mika_alternative_terminal_contracts(_source: &str) -> ParseOutcome {
    ParseOutcome::Failure
}

fn repl_value(command: ReplCommand) -> Value {
    match command {
        ReplCommand::Help => json!({"Help": null}),
        ReplCommand::Quit => json!({"Quit": null}),
        ReplCommand::Save(path) => json!({"Save": path}),
        ReplCommand::Docs(name) => json!({"Docs": name}),
        ReplCommand::Code(code) => {
            let entries = code
                .into_iter()
                .map(|(name, source)| json!([name, serialize(&source)]))
                .collect::<Vec<_>>();
            json!({"Code": entries})
        }
        ReplCommand::Ls => json!({"Ls": null}),
        ReplCommand::Profile(enabled) => json!({"Profile": enabled}),
        ReplCommand::Cd(path) => json!({"Cd": path}),
        ReplCommand::Step(step_id, count) => json!({"Step": [step_id, count]}),
        ReplCommand::Load(paths) => json!({"Load": paths}),
        ReplCommand::Whos(names) => json!({"Whos": names}),
        ReplCommand::Plan => json!({"Plan": null}),
        ReplCommand::Symbols(name) => json!({"Symbols": name}),
        ReplCommand::Clear(name) => json!({"Clear": name}),
        ReplCommand::Clc => json!({"Clc": null}),
    }
}

fn run_repl(source: &str) -> ParseOutcome {
    match parse_repl_command(source) {
        Ok((remaining, command)) => {
            let source_len = source.len();
            let consumed = source_len - remaining.len();
            ParseOutcome::Success {
                ast: repl_value(command),
                consumed,
                source_len,
                remaining: remaining.to_owned(),
                diagnostics: 0,
            }
        }
        Err(_) => ParseOutcome::Failure,
    }
}

fn run_repl_crlf(source: &str) -> ParseOutcome {
    let completed = format!("{source}\r\n");
    run_repl(&completed)
}

fn run_case(entry_point: &str, source: &str) -> ParseOutcome {
    match entry_point {
        "parse" => run_public_parse(source),
        "grammar" => run_public_grammar(source),
        "repl" => run_repl(source),
        "repl-crlf" => run_repl_crlf(source),
        "abstract-el" => run_nom(source, abstract_el),
        "activation-scope" => run_nom(source, activation_scope),
        "alternative-terminal-contracts" => run_alternative_terminal_contracts(source),
        "body" => run_nom(source, body),
        "boolean" => run_nom(source, boolean),
        "box-drawing-char" => run_nom(source, box_drawing_char),
        "box-drawing-emoji" => run_nom(source, box_drawing_emoji),
        "citation" => run_nom(source, citation),
        "code-block" => run_nom(source, code_block),
        "comment" => run_nom(source, comment),
        "emoji" => run_nom(source, emoji),
        "equation" => run_nom(source, equation),
        "error-block" => run_nom(source, error_block),
        "escaped-char" => run_nom(source, escaped_char),
        "eval-inline-mech-code" => run_nom(source, eval_inline_mech_code),
        "expression" => run_nom(source, expression),
        "fancy-table" => run_nom(source, fancy_table),
        "figures" => run_nom(source, figures),
        "float" => run_nom(source, float),
        "footnote" => run_nom(source, footnote),
        "forbidden-emoji" => run_nom(source, forbidden_emoji),
        "fsm-async-transition" => run_nom(source, fsm_async_transition),
        "fsm-block-transition" => run_nom(source, fsm_block_transition),
        "fsm-comment-arm" => run_nom(source, fsm_comment_arm),
        "fsm-declare" => run_nom(source, fsm_declare),
        "fsm-implementation" => run_nom(source, fsm_implementation),
        "fsm-instance" => run_nom(source, fsm_instance),
        "fsm-output" => run_nom(source, fsm_output),
        "fsm-specification" => run_nom(source, fsm_specification),
        "fsm-state-transition" => run_nom(source, fsm_state_transition),
        "fsm-statement-transition" => run_nom(source, fsm_statement_transition),
        "fsm-transition" => run_nom(source, fsm_transition),
        "function-define" => run_nom(source, function_define),
        "gen-operator" => run_nom(source, gen_operator),
        "grouping-symbol" => run_nom(source, grouping_symbol),
        "identifier" => run_nom(source, identifier),
        "idea-block" => run_nom(source, idea_block),
        "image" => run_nom(source, img),
        "info-block" => run_nom(source, info_block),
        "inline-mech-code" => run_nom(source, inline_mech_code),
        "kind-annotation" => run_nom(source, kind_annotation),
        "literal" => run_nom(source, literal),
        "match-expression" => run_nom(source, match_expression),
        "mech-code" => run_nom(source, mech_code),
        "mechdown-list" => run_nom(source, mechdown_list),
        "mechdown-table" => run_nom(source, mechdown_table),
        #[cfg(feature = "mika")]
        "mika" => run_nom(source, mika),
        #[cfg(not(feature = "mika"))]
        "mika" => ParseOutcome::Failure,
        "mika-alternative-terminal-contracts" => run_mika_alternative_terminal_contracts(source),
        #[cfg(feature = "mika")]
        "mika-expression-inner" => run_nom(source, mika_expression_inner),
        #[cfg(not(feature = "mika"))]
        "mika-expression-inner" => ParseOutcome::Failure,
        #[cfg(feature = "mika")]
        "mika-eye-left" => run_nom(source, mika_eye_left),
        #[cfg(not(feature = "mika"))]
        "mika-eye-left" => ParseOutcome::Failure,
        #[cfg(feature = "mika")]
        "mika-eye-right" => run_nom(source, mika_eye_right),
        #[cfg(not(feature = "mika"))]
        "mika-eye-right" => ParseOutcome::Failure,
        #[cfg(feature = "mika")]
        "mini-mika" => run_nom(source, mini_mika),
        #[cfg(not(feature = "mika"))]
        "mini-mika" => ParseOutcome::Failure,
        "module-import" => run_nom(source, module_import),
        "newline-indent" => run_nom(source, newline_indent),
        "normalized-citation" => run_normalized_nom(source, citation),
        "normalized-footnote" => run_normalized_nom(source, footnote),
        "normalized-paragraph-newline" => run_normalized_nom(source, paragraph_newline),
        "number" => run_nom(source, number),
        "paragraph" => run_nom(source, paragraph),
        "paragraph-newline" => run_nom(source, paragraph_newline),
        "paragraph-text" => run_nom(source, paragraph_text),
        "pattern" => run_nom(source, pattern),
        "prompt" => run_nom(source, prompt),
        "question-block" => run_nom(source, question_block),
        "quote-block" => run_nom(source, quote_block),
        "regular-table" => run_nom(source, regular_table),
        "section" => run_nom(source, section),
        "section-element" => run_nom(source, section_element),
        "slice" => run_nom(source, slice),
        "space-tab1" => run_nom(source, space_tab1),
        "statement" => run_nom(source, statement),
        "strike-sigil" => run_nom(source, strike_sigil),
        "string" => run_nom(source, string),
        "structure" => run_nom(source, structure),
        "subscript" => run_nom(source, subscript),
        "subtitle" => run_nom(source, subtitle),
        "success-block" => run_nom(source, success_block),
        "synth-operator" => run_nom(source, synth_operator),
        "table-column" => run_nom(source, table_column),
        "table-horz" => run_nom(source, table_horz),
        "terminal-contracts" => run_terminal_contracts(source),
        "thematic-break" => run_nom(source, thematic_break),
        "title" => run_nom(source, title),
        "ul-subtitle" => run_nom(source, ul_subtitle),
        "underline-sigil" => run_nom(source, underline_sigil),
        "warning-block" => run_nom(source, warning_block),
        "xor-operator" => run_nom(source, xor),
        other => panic!("unsupported entry point {other:?}"),
    }
}

fn contains_error_variant(value: &Value) -> bool {
    match value {
        Value::Array(values) => values.iter().any(contains_error_variant),
        Value::Object(object) => {
            object.contains_key("Error") || object.values().any(contains_error_variant)
        }
        _ => false,
    }
}

fn snapshot_value(
    case: &Case,
    ast: Value,
    consumed: usize,
    source_len: usize,
    remaining: String,
) -> Value {
    let mut value = json!({
        "ast": ast,
        "consumption": {
            "consumed_source_graphemes": consumed,
            "remaining_source": remaining,
            "source_graphemes": source_len,
        },
        "entry_point": case.entry_point,
        "rule": case.rule,
    });
    normalize(&mut value);
    value
}

fn assert_snapshot(case: &Case, actual: &Value) {
    assert_ne!(
        case.snapshot_file, "-",
        "{} must name a snapshot file",
        case.id
    );
    let path = fixture_root().join(&case.snapshot_file);
    let actual_text = format!(
        "{}\n",
        serde_json::to_string_pretty(actual).expect("snapshot must serialize")
    );
    if std::env::var_os("MECH_UPDATE_SNAPSHOTS").is_some() {
        fs::write(&path, actual_text)
            .unwrap_or_else(|error| panic!("failed to update {}: {error}", path.display()));
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    assert_eq!(actual_text, expected, "snapshot mismatch for {}", case.id);
}

#[test]
fn grammar_cases_manifest_is_well_formed() {
    let root = fixture_root();
    let cases = read_cases();
    assert!(!cases.is_empty(), "cases.tsv must contain cases");

    let mut ids = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut snapshots = BTreeSet::new();
    for case in &cases {
        assert!(ids.insert(case.id.clone()), "duplicate case id {}", case.id);
        assert!(!case.rule.is_empty(), "{} has no grammar rule", case.id);
        assert!(
            ENTRY_POINTS.contains(&case.entry_point.as_str()),
            "{} has unknown entry point {}",
            case.id,
            case.entry_point
        );
        assert!(
            EXPECTED_RESULTS.contains(&case.expected_result.as_str()),
            "{} has unknown expected result {}",
            case.id,
            case.expected_result
        );
        assert!(
            matches!(
                case.feature_set.as_str(),
                "all" | "default" | "base" | "invariant-define" | "mika" | "mika-disabled"
            ),
            "{} has unknown feature set {}",
            case.id,
            case.feature_set
        );
        assert!(
            !case.notes.is_empty(),
            "{} must explain its boundary",
            case.id
        );

        let source = root.join(&case.source_file);
        assert!(source.is_file(), "{} source fixture is missing", case.id);
        assert!(
            sources.insert(case.source_file.clone()),
            "{} shares source fixture {}; use one source per case",
            case.id,
            case.source_file
        );
        let expected_dir = if matches!(
            case.expected_result.as_str(),
            "accept" | "accept-prefix" | "recover-with-error"
        ) {
            "accepted"
        } else {
            "rejected"
        };
        assert!(
            case.source_file.starts_with(&format!("{expected_dir}/")),
            "{} source must live under {expected_dir}/",
            case.id
        );

        if matches!(
            case.expected_result.as_str(),
            "accept" | "accept-prefix" | "recover-with-error"
        ) {
            assert!(
                case.snapshot_file.starts_with("snapshots/"),
                "{} accepted behavior must have an ordinary JSON snapshot",
                case.id
            );
            assert!(
                snapshots.insert(case.snapshot_file.clone()),
                "snapshot {} is shared by multiple cases",
                case.snapshot_file
            );
            if std::env::var_os("MECH_UPDATE_SNAPSHOTS").is_none() {
                assert!(
                    root.join(&case.snapshot_file).is_file(),
                    "{} snapshot is missing",
                    case.id
                );
            }
        } else {
            assert_eq!(
                case.snapshot_file, "-",
                "{} rejected behavior must not snapshot diagnostics",
                case.id
            );
        }
    }

    assert_eq!(
        sources,
        relative_files(&root, &["accepted", "rejected"]),
        "cases.tsv and source fixtures must have exact parity"
    );
    assert_eq!(
        snapshots,
        relative_files(&root, &["snapshots"]),
        "cases.tsv and snapshot fixtures must have exact parity"
    );

    // Guard the Phase 0 boundary cases explicitly.
    for required in [
        "LEX-DASH-TIGHT",
        "LEX-DASH-SPACED",
        "LEX-SLASH-TIGHT",
        "LEX-SLASH-SPACED",
        "LEX-RATIONAL-TIGHT",
        "LEX-RATIONAL-SPACED",
        "PAR-COLON-EQUAL-SEPARATE",
        "INCOMPLETE-MECH-CODE",
        "INCOMPLETE-PARAGRAPH",
        "INCOMPLETE-PUBLIC-PARSE",
        "SLASH-EXPR-AFTER",
        "SLASH-EXPR-BEFORE",
        "RATIONAL-AFTER",
        "RATIONAL-BEFORE",
        "OP-ADD-TIGHT",
        "OP-ADD-SPACED",
        "OP-ADD-NEWLINE",
        "OP-DELTA-TIGHT",
        "OP-DELTA-SPACED",
        "OP-DELTA-NEWLINE",
        "SOURCE-NO-FINAL-NEWLINE",
        "SOURCE-ONE-FINAL-NEWLINE",
        "SOURCE-TWO-FINAL-NEWLINES",
        "PAR-HTTP-ROUTING",
        "PERIOD-FUNCTION-MATCH-OPTIONAL",
        "PERIOD-FSM-SPEC-REQUIRED",
        "EXPR-FSM-PIPE",
        "EXPR-SET-COMPREHENSION",
        "EXPR-MATRIX-COMPREHENSION",
        "EXPR-RANGE",
        "EXPR-FORMULA",
        "PATTERN-MALFORMED-PREFIX",
        "SECTION-MECH",
        "SECTION-PARAGRAPH",
        "ALT-BEST-MECH-CODE",
        "ALT-BEST-SECTION-ELEMENT",
        "GRAMMAR-WHITESPACE-STRIPPING",
        "INVARIANT-DEFINE",
        "MIKA-ALTERNATIVE-TERMINAL-CONTRACTS",
        "MIKA-CLOSE-SECTION-ZERO-CONSUMPTION",
        "LOWER-GEN-OPERATOR",
        "LOWER-NEWLINE-INDENT",
        "LOWER-STRIKE-SIGIL",
        "LOWER-SYNTH-OPERATOR",
        "LOWER-TABLE-COLUMN",
        "LOWER-TABLE-HORZ",
        "LOWER-UNDERLINE-SIGIL",
        "ALTERNATIVE-TERMINAL-CONTRACTS",
        "TERMINAL-CONTRACTS",
    ] {
        assert!(ids.contains(required), "missing required case {required}");
    }

    let covered_rules = cases
        .iter()
        .map(|case| case.rule.as_str())
        .collect::<BTreeSet<_>>();
    for family in [
        "activation-scope",
        "atom",
        "boolean",
        "citation",
        "code-block",
        "comment",
        "complete-document",
        "fenced-mech",
        "fsm-declare",
        "fsm-implementation",
        "fsm-specification",
        "function-define",
        "function-match-arm",
        "grammar",
        "identifier",
        "image",
        "inline-mech-code",
        "kind-annotation",
        "match-expression",
        "mechdown-list",
        "mechdown-table",
        "module-import",
        "number",
        "paragraph",
        "pattern",
        "repl-command",
        "statement",
        "string",
        "structure",
        "subscript",
        "title",
    ] {
        assert!(
            covered_rules.contains(family),
            "required grammar family {family:?} has no conformance case"
        );
    }
}

fn split_macro_arguments(arguments: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in arguments.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
        } else if character == '"' {
            in_string = true;
        } else if character == ',' {
            result.push(arguments[start..index].trim());
            start = index + character.len_utf8();
        }
    }
    result.push(arguments[start..].trim());
    result
}

fn rust_function_body<'source>(source: &'source str, function_name: &str) -> &'source str {
    let public_needle = format!("pub fn {function_name}(");
    let private_needle = format!("fn {function_name}(");
    let function_start = source
        .find(&public_needle)
        .or_else(|| source.find(&private_needle))
        .unwrap_or_else(|| panic!("missing parser function {function_name}"));
    let opening_offset = source[function_start..]
        .find('{')
        .unwrap_or_else(|| panic!("missing body for parser function {function_name}"));
    let opening = function_start + opening_offset;

    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (offset, character) in source[opening..].char_indices() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = Some(character);
        } else if character == '{' {
            depth += 1;
        } else if character == '}' {
            depth -= 1;
            if depth == 0 {
                return &source[opening + 1..opening + offset];
            }
        }
    }
    panic!("unclosed body for parser function {function_name}");
}

fn normalize_alt_branch(branch: &str) -> String {
    let without_comments = branch
        .lines()
        .map(|line| line.split("//").next().unwrap_or_default())
        .collect::<String>();
    let compact = without_comments
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if let Some(literal) = compact
        .strip_prefix("tag(\"")
        .and_then(|literal| literal.strip_suffix("\")"))
    {
        format!("tag:{literal}")
    } else {
        compact
    }
}

fn first_alt_branches(source: &str, function_name: &str) -> BTreeSet<String> {
    let body = rust_function_body(source, function_name);
    let alt_start = body
        .find("alt((")
        .unwrap_or_else(|| panic!("{function_name} has no alt tuple"))
        + "alt((".len();
    let alternatives = &body[alt_start..];
    let mut result = BTreeSet::new();
    let mut branch_start = 0usize;
    let mut delimiter_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, character) in alternatives.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' | '[' | '{' => delimiter_depth += 1,
            ')' if delimiter_depth == 0 => {
                let branch = normalize_alt_branch(&alternatives[branch_start..offset]);
                if !branch.is_empty() {
                    assert!(
                        result.insert(branch),
                        "{function_name} has a duplicate direct alternative"
                    );
                }
                return result;
            }
            ')' | ']' | '}' => delimiter_depth -= 1,
            ',' if delimiter_depth == 0 => {
                let branch = normalize_alt_branch(&alternatives[branch_start..offset]);
                if !branch.is_empty() {
                    assert!(
                        result.insert(branch),
                        "{function_name} has a duplicate direct alternative"
                    );
                }
                branch_start = offset + character.len_utf8();
            }
            _ => {}
        }
    }
    panic!("unclosed alt tuple in parser function {function_name}");
}

fn strings(values: impl IntoIterator<Item = &'static str>) -> BTreeSet<String> {
    values.into_iter().map(str::to_owned).collect()
}

#[test]
fn alternative_terminal_contracts_match_source() {
    let syntax_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let base_source =
        fs::read_to_string(syntax_root.join("base.rs")).expect("base.rs must be readable");
    let literals_source =
        fs::read_to_string(syntax_root.join("literals.rs")).expect("literals.rs must be readable");
    let structures_source = fs::read_to_string(syntax_root.join("structures.rs"))
        .expect("structures.rs must be readable");
    let state_machine_source = fs::read_to_string(syntax_root.join("state_machines.rs"))
        .expect("state_machines.rs must be readable");
    let patterns_source =
        fs::read_to_string(syntax_root.join("patterns.rs")).expect("patterns.rs must be readable");
    let mechdown_source =
        fs::read_to_string(syntax_root.join("mechdown.rs")).expect("mechdown.rs must be readable");
    let statements_source = fs::read_to_string(syntax_root.join("statements.rs"))
        .expect("statements.rs must be readable");
    let expressions_source = fs::read_to_string(syntax_root.join("expressions.rs"))
        .expect("expressions.rs must be readable");

    let mut contract_branches = BTreeMap::<&str, BTreeSet<String>>::new();
    for contract in ALTERNATIVE_TOKEN_CONTRACTS {
        contract_branches
            .entry(contract.wrapper)
            .or_default()
            .insert(contract.direct_branch.to_owned());
    }
    for contract in UNIT_ALTERNATIVE_CONTRACTS
        .iter()
        .filter(|contract| contract.wrapper == "guard_operator")
    {
        contract_branches
            .entry(contract.wrapper)
            .or_default()
            .insert(contract.direct_branch.to_owned());
    }

    for (source, function_name) in [
        (&base_source, "transition_operator"),
        (&base_source, "output_operator"),
        (&base_source, "grouping_symbol"),
        (&base_source, "punctuation"),
        (&base_source, "escaped_char"),
        (&base_source, "symbol"),
        (&base_source, "text"),
        (&base_source, "raw_text"),
        (&base_source, "new_line"),
        (&base_source, "space_tab"),
        (&literals_source, "left_angle"),
        (&literals_source, "right_angle"),
        (&structures_source, "matrix_start"),
        (&structures_source, "matrix_end"),
        (&structures_source, "box_drawing_char"),
        (&structures_source, "box_drawing_emoji"),
        (&structures_source, "table_start"),
        (&structures_source, "table_end"),
        (&structures_source, "table_separator"),
        (&structures_source, "table_horz"),
        (&state_machine_source, "guard_operator"),
    ] {
        assert_eq!(
            contract_branches
                .get(function_name)
                .unwrap_or_else(|| panic!("missing alternative contract for {function_name}")),
            &first_alt_branches(source, function_name),
            "{function_name} source alternatives drifted from its executable contract"
        );
    }

    let nested_leaf_branches = |wrapper: &str, direct_branch: &str| {
        ALTERNATIVE_TOKEN_CONTRACTS
            .iter()
            .filter(|contract| {
                contract.wrapper == wrapper && contract.direct_branch == direct_branch
            })
            .map(|contract| contract.leaf_branch.to_owned())
            .collect::<BTreeSet<_>>()
    };
    for (wrapper, direct_branch, source, nested_function) in [
        (
            "grouping_symbol",
            "left_angle",
            &literals_source,
            "left_angle",
        ),
        (
            "grouping_symbol",
            "right_angle",
            &literals_source,
            "right_angle",
        ),
        ("escaped_char", "symbol", &base_source, "symbol"),
        ("escaped_char", "punctuation", &base_source, "punctuation"),
        (
            "table_start",
            "table_separator",
            &structures_source,
            "table_separator",
        ),
        (
            "table_end",
            "table_separator",
            &structures_source,
            "table_separator",
        ),
    ] {
        assert_eq!(
            nested_leaf_branches(wrapper, direct_branch),
            first_alt_branches(source, nested_function),
            "{wrapper}:{direct_branch} nested alternatives drifted from source"
        );
    }

    assert_eq!(
        SPREAD_ALTERNATIVE_CONTRACTS
            .iter()
            .map(|(branch, _, _)| (*branch).to_owned())
            .collect::<BTreeSet<_>>(),
        first_alt_branches(&patterns_source, "spread_operator")
    );
    assert_eq!(
        CODEBLOCK_SIGIL_CONTRACTS
            .iter()
            .map(|(branch, _, _)| (*branch).to_owned())
            .collect::<BTreeSet<_>>(),
        first_alt_branches(&mechdown_source, "codeblock_sigil")
    );
    assert_eq!(
        FLOAT_SIGIL_CONTRACTS
            .iter()
            .map(|(branch, _, _)| (*branch).to_owned())
            .collect::<BTreeSet<_>>(),
        first_alt_branches(&mechdown_source, "float_sigil")
    );
    assert_eq!(
        OP_ASSIGN_CONTRACTS
            .iter()
            .map(|(branch, _, _)| (*branch).to_owned())
            .collect::<BTreeSet<_>>(),
        first_alt_branches(&statements_source, "op_assign_operator")
    );
    assert_eq!(
        GENERATOR_ARROW_CONTRACTS
            .iter()
            .map(|(branch, _, _)| (*branch).to_owned())
            .collect::<BTreeSet<_>>(),
        first_alt_branches(&expressions_source, "generator")
    );
    assert_eq!(
        CHECKED_MARKER_CONTRACTS
            .iter()
            .filter(|(_, _, checked)| *checked)
            .map(|(branch, _, _)| (*branch).to_owned())
            .collect::<BTreeSet<_>>(),
        first_alt_branches(&mechdown_source, "checked_item")
    );
    assert_eq!(
        SUCCESS_ERROR_SIGIL_CONTRACTS
            .iter()
            .filter(|(_, _, success)| *success)
            .map(|(branch, _, _)| (*branch).to_owned())
            .collect::<BTreeSet<_>>(),
        first_alt_branches(&mechdown_source, "success_block")
    );
    assert_eq!(
        SUCCESS_ERROR_SIGIL_CONTRACTS
            .iter()
            .filter(|(_, _, success)| !*success)
            .map(|(branch, _, _)| (*branch).to_owned())
            .collect::<BTreeSet<_>>(),
        first_alt_branches(&mechdown_source, "error_block")
    );

    let compact_identifier = rust_function_body(&base_source, "identifier")
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    assert!(compact_identifier.contains("alt((alpha_token,emoji))"));
    assert!(compact_identifier.contains("alt((alpha_token,digit_token,identifier_symbol,emoji))"));
    assert_eq!(
        IDENTIFIER_ALTERNATIVE_CONTRACTS
            .iter()
            .map(|(category, _)| *category)
            .collect::<Vec<_>>(),
        vec!["alpha-start-with-every-symbol-rest", "emoji-start"]
    );
    assert!(
        rust_function_body(&base_source, "space_tab1")
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>()
            .contains("many1(space_tab)")
    );
    assert!(
        rust_function_body(&mechdown_source, "unchecked_item")
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>()
            .contains("whitespace0(input)")
    );
    assert_eq!(
        strings(["alpha:n", "alpha:t", "alpha:r", "alpha:x"]),
        ALTERNATIVE_TOKEN_CONTRACTS
            .iter()
            .filter(|contract| {
                contract.wrapper == "escaped_char" && contract.direct_branch == "alpha_token"
            })
            .map(|contract| contract.leaf_branch.to_owned())
            .collect(),
        "escaped-char must probe the three transformations and an ordinary alpha"
    );
}

#[cfg(feature = "mika")]
fn rust_enum_variants(source: &str, enum_name: &str) -> BTreeSet<String> {
    let declaration = format!("pub enum {enum_name}");
    let start = source
        .find(&declaration)
        .unwrap_or_else(|| panic!("missing enum {enum_name}"));
    let opening = start
        + source[start..]
            .find('{')
            .unwrap_or_else(|| panic!("missing body for enum {enum_name}"));
    let closing = opening
        + source[opening..]
            .find('}')
            .unwrap_or_else(|| panic!("unclosed enum {enum_name}"));
    source[opening + 1..closing]
        .lines()
        .filter_map(|line| {
            let variant = line
                .split("//")
                .next()
                .expect("split always returns one field")
                .trim()
                .trim_end_matches(',')
                .trim();
            (!variant.is_empty()).then(|| variant.to_owned())
        })
        .collect()
}

#[cfg(feature = "mika")]
fn const_variant_references(
    source: &str,
    constant_name: &str,
    enum_name: &str,
) -> BTreeSet<String> {
    let declaration = format!("const {constant_name}");
    let start = source
        .find(&declaration)
        .unwrap_or_else(|| panic!("missing constant {constant_name}"));
    let end = start
        + source[start..]
            .find("];")
            .unwrap_or_else(|| panic!("unclosed constant {constant_name}"));
    let body = &source[start..end];
    let prefix = format!("{enum_name}::");
    let mut variants = BTreeSet::new();
    let mut rest = body;
    while let Some(offset) = rest.find(&prefix) {
        rest = &rest[offset + prefix.len()..];
        let length = rest
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .map(char::len_utf8)
            .sum::<usize>();
        assert!(length > 0, "malformed {enum_name} variant reference");
        variants.insert(rest[..length].to_owned());
        rest = &rest[length..];
    }
    variants
}

#[cfg(feature = "mika")]
fn debug_variant_names<T: std::fmt::Debug>(values: &[T]) -> BTreeSet<String> {
    values.iter().map(|value| format!("{value:?}")).collect()
}

#[cfg(feature = "mika")]
#[test]
fn mika_alternative_terminal_contracts_match_source() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let syntax_source = fs::read_to_string(manifest_dir.join("src/mika.rs"))
        .expect("syntax mika.rs must be readable");
    let core_source = fs::read_to_string(manifest_dir.join("../core/src/mika.rs"))
        .expect("core mika.rs must be readable");

    assert_eq!(
        MIKA_ARM_LEFT_CONTRACTS
            .iter()
            .map(|(literal, _)| format!("tag:{literal}"))
            .collect::<BTreeSet<_>>(),
        first_alt_branches(&syntax_source, "mika_arm_left")
    );
    assert_eq!(
        MIKA_ARM_RIGHT_CONTRACTS
            .iter()
            .map(|(literal, _)| format!("tag:{literal}"))
            .collect::<BTreeSet<_>>(),
        first_alt_branches(&syntax_source, "mika_arm_right")
    );

    let arm_contract_variants = MIKA_ARM_LEFT_CONTRACTS
        .iter()
        .chain(MIKA_ARM_RIGHT_CONTRACTS)
        .map(|(_, variant)| format!("{variant:?}"))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        arm_contract_variants,
        rust_enum_variants(&core_source, "MikaArm"),
        "every MikaArm enum variant must be exercised by a side parser"
    );

    let left_variants = debug_variant_names(MIKA_EYE_LEFT_CONTRACTS);
    assert_eq!(
        left_variants,
        rust_enum_variants(&core_source, "MikaEyeLeft")
    );
    assert_eq!(
        left_variants,
        const_variant_references(&syntax_source, "LEFT_EYE_ORDER", "MikaEyeLeft")
    );

    let right_variants = debug_variant_names(MIKA_EYE_RIGHT_CONTRACTS);
    assert_eq!(
        right_variants,
        rust_enum_variants(&core_source, "MikaEyeRight")
    );
    assert_eq!(
        right_variants,
        const_variant_references(&syntax_source, "RIGHT_EYE_ORDER", "MikaEyeRight")
    );

    let nose_variants = MIKA_NOSE_CONTRACTS
        .iter()
        .map(|(declared, _)| format!("{declared:?}"))
        .collect::<BTreeSet<_>>();
    assert_eq!(nose_variants, rust_enum_variants(&core_source, "MikaNose"));
    assert_eq!(
        nose_variants,
        const_variant_references(&syntax_source, "NOSE_ORDER", "MikaNose")
    );
    assert_eq!(
        MIKA_NOSE_CONTRACTS
            .iter()
            .filter(|(declared, expected)| declared != expected)
            .map(|(declared, expected)| (declared.symbol(), format!("{expected:?}")))
            .collect::<Vec<_>>(),
        vec![("⦾", "Stage2".to_owned())],
        "the Stage2/Stage3 shared-symbol first-match behavior must remain explicit"
    );
}

#[test]
fn fixed_terminal_contracts_match_source_and_inventory() {
    let contract_source_signatures = FIXED_TERMINAL_CONTRACTS
        .iter()
        .map(|contract| {
            format!(
                "{}\t{}\t{}\t{}",
                contract.function_name,
                contract.literal_source,
                contract.kind_name,
                contract.parser_macro
            )
        })
        .collect::<BTreeSet<_>>();
    let contract_inventory_signatures = FIXED_TERMINAL_CONTRACTS
        .iter()
        .map(|contract| {
            format!(
                "{}\t{}\t{}\t{}\t{}",
                contract.production_id,
                contract.function_name,
                contract.literal_source,
                contract.kind_name,
                contract.parser_macro
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        contract_source_signatures.len(),
        FIXED_TERMINAL_CONTRACTS.len(),
        "fixed-terminal contract has a duplicate function"
    );
    assert_eq!(
        contract_inventory_signatures.len(),
        FIXED_TERMINAL_CONTRACTS.len(),
        "fixed-terminal contract has a duplicate inventory id"
    );
    assert_eq!(
        FIXED_TERMINAL_CONTRACTS
            .iter()
            .filter(|contract| contract.parser_macro == "ws0_leaf")
            .count(),
        13,
        "fixed-terminal contract must probe every ws0_leaf! parser"
    );

    let base_source =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/base.rs"))
            .expect("base.rs must be readable");
    let source_signatures = base_source
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let (parser_macro, invocation) = if let Some(invocation) = line.strip_prefix("leaf!") {
                ("leaf", invocation)
            } else if let Some(invocation) = line.strip_prefix("ws0_leaf!") {
                ("ws0_leaf", invocation)
            } else {
                return None;
            };
            let open = invocation
                .find(['{', '('])
                .unwrap_or_else(|| panic!("malformed {parser_macro}! invocation: {line}"));
            let close_delimiter = match invocation.as_bytes()[open] {
                b'{' => '}',
                b'(' => ')',
                _ => unreachable!(),
            };
            let close = invocation
                .rfind(close_delimiter)
                .unwrap_or_else(|| panic!("unclosed {parser_macro}! invocation: {line}"));
            let fields = split_macro_arguments(&invocation[open + 1..close]);
            assert_eq!(
                fields.len(),
                3,
                "malformed {parser_macro}! invocation: {line}"
            );
            let kind = fields[2].strip_prefix("TokenKind::").unwrap_or_else(|| {
                panic!("missing TokenKind in {parser_macro}! invocation: {line}")
            });
            Some(format!(
                "{}\t{}\t{}\t{}",
                fields[0], fields[1], kind, parser_macro
            ))
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        contract_source_signatures, source_signatures,
        "fixed-terminal contract and base.rs macro invocations drifted"
    );

    let inventory_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/design/grammar-audit/productions.tsv");
    let inventory_source = fs::read_to_string(&inventory_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", inventory_path.display()));
    let mut lines = inventory_source.lines();
    let header = lines
        .next()
        .expect("productions.tsv must have a header")
        .split('\t')
        .collect::<Vec<_>>();
    let column = |name: &str| {
        header
            .iter()
            .position(|column| *column == name)
            .unwrap_or_else(|| panic!("productions.tsv is missing column {name:?}"))
    };
    let id = column("id");
    let module = column("module");
    let rust_function = column("rust-function");
    let classification = column("classification");
    let feature_gate = column("feature-gate");
    let conformance_cases = column("conformance-cases");
    let implementation_path = column("implementation-path");
    let notes = column("notes");
    let note_prefix = "Macro-generated terminal ";
    let inventory_signatures = lines
        .filter_map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if !fields[notes].starts_with(note_prefix) {
                return None;
            }
            assert_eq!(
                fields[module], "base",
                "{} is a macro terminal outside base.rs",
                fields[id]
            );
            assert_eq!(
                fields[classification], "terminal",
                "{} has the wrong classification",
                fields[id]
            );
            assert_eq!(
                fields[feature_gate], "always",
                "{} has a feature-gated fixed-terminal parser",
                fields[id]
            );
            assert_eq!(
                fields[conformance_cases], "TERMINAL-CONTRACTS",
                "{} must map to the fixed-terminal contract case",
                fields[id]
            );
            assert_eq!(
                fields[implementation_path],
                format!("src/syntax/src/base.rs::{}", fields[rust_function]),
                "{} has a stale implementation path",
                fields[id]
            );
            let note = fields[notes]
                .strip_prefix(note_prefix)
                .expect("note prefix was checked");
            let (literal_source, kind_and_spacing) = note
                .split_once(" with TokenKind::")
                .unwrap_or_else(|| panic!("{} has a malformed terminal note", fields[id]));
            let (kind, spacing) = kind_and_spacing
                .split_once("; ")
                .unwrap_or_else(|| panic!("{} has a malformed terminal note", fields[id]));
            let parser_macro = match spacing {
                "no implicit whitespace." => "leaf",
                "whitespace0 on both sides." => "ws0_leaf",
                other => panic!(
                    "{} has unknown terminal whitespace note {other:?}",
                    fields[id]
                ),
            };
            Some(format!(
                "{}\t{}\t{}\t{}\t{}",
                fields[id], fields[rust_function], literal_source, kind, parser_macro
            ))
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        contract_inventory_signatures, inventory_signatures,
        "fixed-terminal contract and macro-generated inventory rows drifted"
    );
}

#[test]
fn production_conformance_mapping_is_complete() {
    let cases = read_cases();
    let cases_by_id = cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<BTreeMap<_, _>>();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/design/grammar-audit/productions.tsv");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let mut lines = source.lines();
    let header = lines
        .next()
        .expect("productions.tsv must have a header")
        .split('\t')
        .collect::<Vec<_>>();
    let column = |name: &str| {
        header
            .iter()
            .position(|column| *column == name)
            .unwrap_or_else(|| panic!("productions.tsv is missing column {name:?}"))
    };
    let id = column("id");
    let grammar_name = column("grammar-name");
    let classification = column("classification");
    let feature_gate = column("feature-gate");
    let conformance_cases = column("conformance-cases");
    let exempt_roles = [
        "diagnostic",
        "helper",
        "not-grammar",
        "parser-control",
        "recovery",
        "semantic-validation",
    ];

    let mut mapped = 0;
    let mut exempt = 0;
    for (offset, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(
            fields.len(),
            header.len(),
            "{}:{} has the wrong number of columns",
            path.display(),
            offset + 2
        );
        let production_id = fields[id];
        let mapping = fields[conformance_cases];
        assert_ne!(
            mapping, "pending-phase-0c",
            "{production_id} has no conformance mapping"
        );

        if mapping == "not-applicable" {
            exempt += 1;
            assert!(
                fields[grammar_name].is_empty(),
                "{production_id} is grammar-bearing and cannot be exempt"
            );
            assert!(
                fields[classification]
                    .split('/')
                    .all(|role| exempt_roles.contains(&role)),
                "{production_id} uses a non-mechanics exemption classification {:?}",
                fields[classification]
            );
            continue;
        }

        mapped += 1;
        assert!(
            !mapping.is_empty(),
            "{production_id} has an empty conformance mapping"
        );
        let required_feature = match fields[feature_gate] {
            "always" => "all",
            "invariant_define" => "invariant-define",
            "mika" => "mika",
            other => panic!("{production_id} has unknown feature gate {other:?}"),
        };
        for case_id in mapping.split(',').map(str::trim) {
            let case = cases_by_id.get(case_id).unwrap_or_else(|| {
                panic!("{production_id} references missing conformance case {case_id:?}")
            });
            assert_ne!(
                case.expected_result, "feature-disabled",
                "{production_id} maps to an absence-only case {case_id}"
            );
            assert_eq!(
                case.feature_set, required_feature,
                "{production_id} ({}) maps to feature-incompatible case {case_id} ({})",
                fields[feature_gate], case.feature_set
            );
        }
    }
    assert!(mapped > 0, "productions.tsv has no mapped grammar rows");
    assert!(exempt > 0, "productions.tsv has no explicit mechanics rows");
}

#[test]
fn grammar_conformance_cases() {
    for case in read_cases() {
        if !enabled_for_this_build(&case.feature_set) {
            continue;
        }
        let source_path = fixture_root().join(&case.source_file);
        let source = read_source(&source_path);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_case(&case.entry_point, &source)
        }))
        .unwrap_or_else(|_| panic!("{} caused the parser to panic", case.id));

        match (case.expected_result.as_str(), outcome) {
            (
                "accept",
                ParseOutcome::Success {
                    ast,
                    consumed,
                    source_len,
                    remaining,
                    ..
                },
            ) => {
                assert_eq!(
                    consumed, source_len,
                    "{} accepted only a prefix; remaining source: {remaining:?}",
                    case.id
                );
                assert!(remaining.is_empty(), "{} left source input", case.id);
                let snapshot = snapshot_value(&case, ast, consumed, source_len, remaining);
                assert_snapshot(&case, &snapshot);
            }
            (
                "accept-prefix",
                ParseOutcome::Success {
                    ast,
                    consumed,
                    source_len,
                    remaining,
                    ..
                },
            ) => {
                assert!(
                    consumed < source_len && !remaining.is_empty(),
                    "{} was expected to demonstrate prefix parsing",
                    case.id
                );
                let snapshot = snapshot_value(&case, ast, consumed, source_len, remaining);
                assert_snapshot(&case, &snapshot);
            }
            (
                "recover-with-error",
                ParseOutcome::Success {
                    ast,
                    consumed,
                    source_len,
                    remaining,
                    diagnostics,
                },
            ) => {
                assert_eq!(
                    consumed, source_len,
                    "{} recovery did not consume the source",
                    case.id
                );
                assert!(
                    diagnostics > 0 || contains_error_variant(&ast),
                    "{} succeeded without recording recovery",
                    case.id
                );
                let snapshot = snapshot_value(&case, ast, consumed, source_len, remaining);
                assert_snapshot(&case, &snapshot);
            }
            ("reject" | "feature-disabled", ParseOutcome::Failure) => {}
            (expected, ParseOutcome::Failure) => {
                panic!("{} expected {expected}, but parsing failed", case.id)
            }
            (expected, ParseOutcome::Success { remaining, .. }) => {
                panic!(
                    "{} expected {expected}, but parsing succeeded completely (remaining {remaining:?})",
                    case.id
                )
            }
        }
    }
}

#[test]
fn grammar_empty_terminal_panic_is_safely_recorded() {
    // `terminal_token` accepts zero inner tokens and then unconditionally
    // unwraps Token::merge_tokens. Phase 0 records this production anomaly
    // without allowing it to abort the conformance process.
    let result = std::panic::catch_unwind(|| parse_grammar(r#"empty := "";"#));
    assert!(
        result.is_err(),
        "the Phase 0 baseline unexpectedly stopped panicking on an empty grammar terminal"
    );
}

#[test]
fn repl_crlf_completion_boundaries() {
    assert!(matches!(
        parse_repl_command(":cd /tmp\r\n"),
        Ok(("", ReplCommand::Cd(path))) if path == "/tmp"
    ));
    // Without CRLF the `cd` branch fails, then ordinary ordered fallback lets
    // the later one-letter `c` code alias consume the complete command.
    assert!(matches!(
        parse_repl_command(":cd /tmp\n"),
        Ok(("", ReplCommand::Code(code))) if code.len() == 1
    ));
    assert!(matches!(
        parse_repl_command(":load one.mec two.mec\r\n"),
        Ok(("", ReplCommand::Load(paths)))
            if paths == ["one.mec".to_owned(), "two.mec".to_owned()]
    ));
    assert!(parse_repl_command(":load one.mec two.mec\n").is_err());
    assert!(matches!(
        parse_repl_command(":help\r\n"),
        Ok(("", ReplCommand::Help))
    ));
    assert!(parse_repl_command(":help\n\n").is_err());
}
