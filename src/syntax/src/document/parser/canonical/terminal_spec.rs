use crate::document::{RuleId, SyntaxKind};

use super::super::rule::rules;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalSpacing {
  Exact,
  Whitespace0Both,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedTerminalSpec {
  pub rule: RuleId,
  pub literal: &'static str,
  pub kind: SyntaxKind,
  pub spacing: TerminalSpacing,
}

macro_rules! fixed_terminal_specs {
  ($(
    $rule:ident => ($literal:literal, $kind:ident, $spacing:ident);
  )+) => {
    pub static FIXED_TERMINALS: &[FixedTerminalSpec] = &[
      $(
        FixedTerminalSpec {
          rule: rules::$rule,
          literal: $literal,
          kind: SyntaxKind::$kind,
          spacing: TerminalSpacing::$spacing,
        },
      )+
    ];
  };
}

fixed_terminal_specs! {
  AMPERSAND => ("&", Ampersand, Exact);
  APOSTROPHE => ("'", Apostrophe, Exact);
  ASTERISK => ("*", Asterisk, Exact);
  AT => ("@", At, Exact);
  BAR => ("|", Bar, Exact);
  BACKSLASH => ("\\", Backslash, Exact);
  CARET => ("^", Caret, Exact);
  COLON => (":", Colon, Exact);
  COMMA => (",", Comma, Exact);
  DASH => ("-", Dash, Exact);
  DOLLAR => ("$", Dollar, Exact);
  EQUAL => ("=", Equal, Exact);
  EXCLAMATION => ("!", Exclamation, Exact);
  GRAVE => ("`", Grave, Exact);
  HASHTAG => ("#", HashTag, Exact);
  NEGATE => ("¬", Not, Exact);
  PERCENT => ("%", Percent, Exact);
  PERIOD => (".", Period, Exact);
  PLUS => ("+", Plus, Exact);
  QUESTION => ("?", Question, Exact);
  QUOTE => ("\"", Quote, Exact);
  SEMICOLON => (";", Semicolon, Exact);
  SLASH => ("/", Slash, Exact);
  TILDE => ("~", Tilde, Exact);
  UNDERSCORE => ("_", Underscore, Exact);
  CHECK_MARK => ("✓", True, Exact);
  CROSS => ("✗", False, Exact);
  ENGLISH_TRUE_LITERAL => ("true", True, Exact);
  ENGLISH_FALSE_LITERAL => ("false", False, Exact);
  SPACE => (" ", Whitespace, Exact);
  NBSP => ("\u{00A0}", Whitespace, Exact);
  THIN_SPACE => ("\u{2009}", Whitespace, Exact);
  NEW_LINE_CHAR => ("\n", Newline, Exact);
  CARRIAGE_RETURN => ("\r", CarriageReturn, Exact);
  CARRIAGE_RETURN_NEW_LINE => ("\r\n", CarriageReturn, Exact);
  TAB => ("\t", Tab, Exact);
  LEFT_BRACKET => ("[", LeftBracket, Exact);
  LEFT_PARENTHESIS => ("(", LeftParen, Exact);
  LEFT_BRACE => ("{", LeftBrace, Exact);
  LEFT_ANGLE1 => ("<", LeftAngle, Exact);
  LEFT_ANGLE2 => ("⟨", LeftAngle, Exact);
  RIGHT_BRACKET => ("]", RightBracket, Exact);
  RIGHT_PARENTHESIS => (")", RightParen, Exact);
  RIGHT_BRACE => ("}", RightBrace, Exact);
  RIGHT_ANGLE1 => (">", RightAngle, Exact);
  RIGHT_ANGLE2 => ("⟩", RightAngle, Exact);
  BOX_TL_ROUND => ("╭", BoxDrawing, Exact);
  BOX_TR_ROUND => ("╮", BoxDrawing, Exact);
  BOX_BL_ROUND => ("╰", BoxDrawing, Exact);
  BOX_BR_ROUND => ("╯", BoxDrawing, Exact);
  BOX_TL_BOLD => ("┏", BoxDrawing, Exact);
  BOX_TR_BOLD => ("┓", BoxDrawing, Exact);
  BOX_BL_BOLD => ("┗", BoxDrawing, Exact);
  BOX_BR_BOLD => ("┛", BoxDrawing, Exact);
  BOX_TL => ("┌", BoxDrawing, Exact);
  BOX_TR => ("┐", BoxDrawing, Exact);
  BOX_BL => ("└", BoxDrawing, Exact);
  BOX_BR => ("┘", BoxDrawing, Exact);
  BOX_CROSS => ("┼", BoxDrawing, Exact);
  BOX_HORZ => ("─", BoxDrawing, Exact);
  BOX_T_LEFT => ("├", BoxDrawing, Exact);
  BOX_T_RIGHT => ("┤", BoxDrawing, Exact);
  BOX_T_TOP => ("┬", BoxDrawing, Exact);
  BOX_T_BOTTOM => ("┴", BoxDrawing, Exact);
  BOX_VERT => ("│", BoxDrawing, Exact);
  BOX_VERT_BOLD => ("┃", BoxDrawing, Exact);
  ABSTRACT_SIGIL => ("%%", AbstractSigil, Exact);
  EMPHASIS_SIGIL => ("*", EmphasisSigil, Exact);
  EQUATION_SIGIL => ("$$", EquationSigil, Exact);
  FOOTNOTE_PREFIX => ("[^", FootnotePrefix, Exact);
  FLOAT_LEFT => ("<<:", FloatLeft, Exact);
  FLOAT_RIGHT => (":>>", FloatRight, Exact);
  HTTP_PREFIX => ("http", HttpPrefix, Exact);
  HIGHLIGHT_SIGIL => ("!!", HighlightSigil, Exact);
  IMG_PREFIX => ("![", ImgPrefix, Exact);
  QUOTE_SIGIL => (">", QuoteSigil, Exact);
  QUESTION_SIGIL => ("(?)>", QuestionSigil, Exact);
  INFO_SIGIL => ("(i)>", InfoSigil, Exact);
  IDEA_SIGIL => ("(*)>", IdeaSigil, Exact);
  WARNING_SIGIL => ("(!)>", WarningSigil, Exact);
  ERROR_SIGIL => ("(x)>", ErrorSigil, Exact);
  ERROR_ALT_SIGIL => ("(✗)>", ErrorSigil, Exact);
  SUCCESS_CHECK_SIGIL => ("(✓)>", SuccessSigil, Exact);
  SUCCESS_SIGIL => ("(+)>", SuccessSigil, Exact);
  STRIKE_SIGIL => ("~~", StrikeSigil, Exact);
  STRONG_SIGIL => ("**", StrongSigil, Exact);
  GRAVE_CODEBLOCK_SIGIL => ("```", GraveCodeBlockSigil, Exact);
  TILDE_CODEBLOCK_SIGIL => ("~~~", TildeCodeBlockSigil, Exact);
  UNDERLINE_SIGIL => ("__", UnderlineSigil, Exact);
  SECTION_SIGIL => ("§", SectionSigil, Exact);
  MIKA_SECTION_OPEN => ("⸢", MikaSectionOpen, Exact);
  MIKA_SECTION_CLOSE => ("⸥", MikaSectionClose, Exact);
  PROMPT_SIGIL => (">:", PromptSigil, Exact);
  IMPORT_SIGIL => ("+>", ModuleImportSigil, Exact);
  MODULE_EXPORT_SIGIL => ("<+", ModuleExportSigil, Exact);
  ASSIGN_OPERATOR => ("=", AssignOperator, Whitespace0Both);
  ASYNC_TRANSITION_OPERATOR => ("~>", AsyncTransitionOperator, Whitespace0Both);
  DEFINE_OPERATOR => (":=", DefineOperatorToken, Whitespace0Both);
  SYNTH_OPERATOR => ("?=", SynthOperator, Whitespace0Both);
  GEN_OPERATOR => ("@=", GenOperator, Whitespace0Both);
  OUTPUT_OPERATOR_A => ("=>", OutputOperator, Whitespace0Both);
  OUTPUT_OPERATOR_U => ("⇒", OutputOperator, Whitespace0Both);
  TRANSITION_OPERATOR_A => ("->", TransitionOperator, Whitespace0Both);
  TRANSITION_OPERATOR_U => ("→", TransitionOperator, Whitespace0Both);
  GENERATOR_ARROW => ("<-", GeneratorArrow, Whitespace0Both);
  GENERATOR_ARROW_U => ("←", GeneratorArrow, Whitespace0Both);
  SPREAD_OPERATOR_A => ("...", SpreadOperator, Whitespace0Both);
  SPREAD_OPERATOR_U => ("…", SpreadOperator, Whitespace0Both);
}

pub const FIXED_TERMINAL_COUNT: usize = 108;

pub fn fixed_terminal_spec(rule: RuleId) -> Option<&'static FixedTerminalSpec> {
  FIXED_TERMINALS.iter().find(|spec| spec.rule == rule)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn table_matches_the_phase_0_fixed_terminal_contract() {
    assert_eq!(FIXED_TERMINALS.len(), FIXED_TERMINAL_COUNT);
    assert_eq!(
      FIXED_TERMINALS
        .iter()
        .filter(|spec| spec.spacing == TerminalSpacing::Whitespace0Both)
        .count(),
      13,
    );
    assert!(FIXED_TERMINALS.iter().all(|spec| !spec.literal.is_empty()));

    for (index, spec) in FIXED_TERMINALS.iter().enumerate() {
      assert!(
        FIXED_TERMINALS[..index]
          .iter()
          .all(|earlier| earlier.rule != spec.rule),
        "duplicate fixed-terminal rule {}",
        spec.rule,
      );
      assert_eq!(fixed_terminal_spec(spec.rule), Some(spec));
    }
  }

  #[test]
  fn duplicate_literals_retain_rule_specific_meanings() {
    let asterisk = fixed_terminal_spec(rules::ASTERISK).unwrap();
    let emphasis = fixed_terminal_spec(rules::EMPHASIS_SIGIL).unwrap();
    assert_eq!(asterisk.literal, emphasis.literal);
    assert_eq!(asterisk.literal, "*");
    assert_eq!(asterisk.kind, SyntaxKind::Asterisk);
    assert_eq!(emphasis.kind, SyntaxKind::EmphasisSigil);

    let equal = fixed_terminal_spec(rules::EQUAL).unwrap();
    let assign = fixed_terminal_spec(rules::ASSIGN_OPERATOR).unwrap();
    assert_eq!(equal.literal, assign.literal);
    assert_eq!(equal.literal, "=");
    assert_eq!(equal.kind, SyntaxKind::Equal);
    assert_eq!(assign.kind, SyntaxKind::AssignOperator);

    let right_angle = fixed_terminal_spec(rules::RIGHT_ANGLE1).unwrap();
    let quote_sigil = fixed_terminal_spec(rules::QUOTE_SIGIL).unwrap();
    assert_eq!(right_angle.literal, quote_sigil.literal);
    assert_eq!(right_angle.literal, ">");
    assert_eq!(right_angle.kind, SyntaxKind::RightAngle);
    assert_eq!(quote_sigil.kind, SyntaxKind::QuoteSigil);
  }

  #[test]
  fn define_operator_uses_the_token_kind_not_the_phase_1_node_kind() {
    let define = fixed_terminal_spec(rules::DEFINE_OPERATOR).unwrap();
    assert_eq!(define.literal, ":=");
    assert_eq!(define.kind, SyntaxKind::DefineOperatorToken);
    assert_eq!(define.spacing, TerminalSpacing::Whitespace0Both);
  }
}
