#[macro_use]
use crate::*;

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  IResult,
  branch::alt,
  sequence::{tuple as nom_tuple, pair},
  combinator::{opt, eof, peek},
  multi::{many1, many_till, many0, separated_list1,separated_list0},
  bytes::complete::{take_until, take_while},
  Err,
  Err::Failure
};

use std::collections::HashMap;
use colored::*;

use crate::*;

// Mika
// ============================================================================

// Speech Bubble
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub struct MikaSpeechBubble {
  pub elements: Vec<SectionElement>,
}

// speech-bubble-text := +(¬(speech-bubble-close | new-line), any) ;
pub fn speech_bubble_text(input: ParseString) -> ParseResult<SpeechBubbleElement> {
  let (input, mut tokens) = many1(tuple((
    is_not(alt((null(speech_bubble_close), null(new_line)))),
    text,
  )))(input)?;
  let mut tokens: Vec<Token> = tokens.into_iter().map(|(_, t)| t).collect();
  let mut merged = Token::merge_tokens(&mut tokens).unwrap();
  merged.kind = TokenKind::Text;
  Ok((input, SpeechBubbleElement::Text(merged)))
}

// speech-bubble-code-line := new-line, ws0, mech-code ;
// A newline inside the bubble after which valid Mech code follows.
pub fn speech_bubble_code_line(input: ParseString) -> ParseResult<SpeechBubbleElement> {
  let (input, _) = new_line(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, code) = mech_code(input)?;
  Ok((input, SpeechBubbleElement::Code(code)))
}

// speech-bubble-element := speech-bubble-code-line | speech-bubble-text ;
// Code is tried first: it has the more specific prefix (new-line + code syntax).
pub fn speech_bubble_element(input: ParseString) -> ParseResult<SpeechBubbleElement> {
  alt((speech_bubble_code_line, speech_bubble_text))(input)
}

// speech-bubble := "⸢", +speech-bubble-element, "⸥" ;
pub fn mika_speech_bubble(input: ParseString) -> ParseResult<MikaSpeechBubble> {
  let msg = "Expects ⸥ to close speech bubble";
  let (input, (_, r)) = range(speech_bubble_open)(input)?;
  let (input, elements) = many1(speech_bubble_element)(input)?;
  let (input, _) = label!(speech_bubble_close, msg, r)(input)?;
  Ok((input, MikaSpeechBubble { elements }))
}

// Mika Statements
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum MikaStatement {
  // ╰⦿╮ ⸢Hello!⸥
  // ╭⦿⌣ ⸢Here is the result:\nx := y[y > 3]⸥
  Standalone {
    mika: Mika,
    bubble: MikaSpeechBubble,
  },
  // z := x ** x'  ─⦿╮ ⸢I added the transpose here.⸥
  Annotation {
    code: Statement,
    mika: Mika,
    bubble: MikaSpeechBubble,
  },
}

// standalone-mika := mika, +space-tab, speech-bubble, ws0 ;
pub fn standalone_mika(input: ParseString) -> ParseResult<MikaStatement> {
  let (input, m)      = mika(input)?;
  let (input, _)      = many1(space_tab)(input)?;
  let (input, bubble) = mika_speech_bubble(input)?;
  let (input, _)      = whitespace0(input)?;
  Ok((input, MikaStatement::Standalone { mika: m, bubble }))
}

// annotated-mika := statement, +space-tab, mika, +space-tab, speech-bubble, ws0 ;
pub fn annotated_mika(input: ParseString) -> ParseResult<MikaStatement> {
  let (input, code)   = statement(input)?;
  let (input, _)      = many1(space_tab)(input)?;
  let (input, m)      = mika(input)?;
  let (input, _)      = many1(space_tab)(input)?;
  let (input, bubble) = mika_speech_bubble(input)?;
  let (input, _)      = whitespace0(input)?;
  Ok((input, MikaStatement::Annotation { code, mika: m, bubble }))
}

// mika-statement := annotated-mika | standalone-mika ;
// Annotated tried first: it requires the longer, more specific prefix.
pub fn mika_statement(input: ParseString) -> ParseResult<MikaStatement> {
  alt((annotated_mika, standalone_mika))(input)
}

// Face / Arm Primitives
// ============================================================================

// mika-nose := "⦿" | "◯" | "⊕" | "∘" | "⦾" | "⊖" | "⦵" | "⊗" | "⏺" | "⍜" | ... ;
pub fn mika_nose(input: ParseString) -> ParseResult<MikaFace> {
  let (input, tok) = alt((
    tag("⦿"), tag("◯"), tag("⊕"), tag("∘"),
    tag("⦾"), tag("⊖"), tag("⦵"), tag("⊗"),
    tag("⏺"), tag("⍜"), tag("⬢"), tag("⬟"), tag("⬣"), tag("⎔"),
  ))(input)?;
  let face = match tok.chars.iter().collect::<String>().as_str() {
    "⦿" => MikaFace::Normal,
    "◯" => MikaFace::Open,
    "⊕" => MikaFace::Back,
    "∘" => MikaFace::Stage1,
    "⦾" => MikaFace::Stage2,
    "⊖" => MikaFace::Blink,
    "⦵" => MikaFace::Wide,
    "⊗" => MikaFace::Error,
    "⏺" => MikaFace::Filled,
    "⍜" => MikaFace::FlatMouth,
    "⬢" => MikaFace::Hexagon,
    "⬟" => MikaFace::Heptagon,
    "⬣" => MikaFace::Octagon,
    "⎔" => MikaFace::OpenHexagon,
    _ => unreachable!(),
  };
  Ok((input, face))
}

// Longer multi-char tokens matched before their single-char prefixes.
pub fn mika_arm_left(input: ParseString) -> ParseResult<MikaArm> {
  let (input, tok) = alt((
    tag("Ɔ∞"),  // BigGripperLeft
    tag("›─"),  // GripperLeft
    tag("›⌣"),  // GestureLeft (decorated)
    tag("·¬"),  // ShootLeft
    tag("-◡"),  // ShrugLeft
    tag("ᗑ"),  // BatWing
    tag("ᕦ"),  // CurlLeft
    tag("~"),   // Dance
    tag("⌣"),  // GestureLeft (bare)
    tag("╭"),  // Left
    tag("⸌"),  // RaisedLeft
    tag("⸸"),  // Sword
    tag("─"),  // Point
    tag("ᓂ"),  // PunchLeft
    tag("ᓇ"),  // PunchLowLeft
    tag("╰"),  // UpLeft
  ))(input)?;
  let arm = match tok.chars.iter().collect::<String>().as_str() {
    "ᗑ"        => MikaArm::BatWing,
    "Ɔ∞"       => MikaArm::BigGripperLeft,
    "ᕦ"        => MikaArm::CurlLeft,
    "~"         => MikaArm::Dance,
    "›⌣" | "⌣" => MikaArm::GestureLeft,
    "›─"        => MikaArm::GripperLeft,
    "╭"         => MikaArm::Left,
    "⸌"         => MikaArm::RaisedLeft,
    "·¬"        => MikaArm::ShootLeft,
    "-◡"        => MikaArm::ShrugLeft,
    "⸸"         => MikaArm::Sword,
    "─"         => MikaArm::Point,
    "ᓂ"         => MikaArm::PunchLeft,
    "ᓇ"         => MikaArm::PunchLowLeft,
    "╰"         => MikaArm::UpLeft,
    _ => unreachable!(),
  };
  Ok((input, arm))
}

pub fn mika_arm_right(input: ParseString) -> ParseResult<MikaArm> {
  let (input, tok) = alt((
    tag("∞C"),  // BigGripperRight
    tag("─‹"),  // GripperRight
    tag("⌣‹"),  // GestureRight (decorated)
    tag("⌐·"),  // ShootRight
    tag("◡-"),  // ShrugRight
    tag("ᗑ"),  // BatWing
    tag("ᕤ"),  // CurlRight
    tag("~"),   // Dance
    tag("⌣"),  // GestureRight (bare)
    tag("╮"),  // Right
    tag("⸍"),  // RaisedRight
    tag("ᗢ"),  // Shield
    tag("─"),  // Point  (NB: also used as left Point; position in the grammar disambiguates)
    tag("ᓀ"),  // PunchRight
    tag("ᓄ"),  // PunchLowRight
    tag("╯"),  // UpRight
  ))(input)?;
  let arm = match tok.chars.iter().collect::<String>().as_str() {
    "ᗑ"         => MikaArm::BatWing,
    "∞C"        => MikaArm::BigGripperRight,
    "ᕤ"         => MikaArm::CurlRight,
    "~"          => MikaArm::Dance,
    "⌣‹" | "⌣"  => MikaArm::GestureRight,
    "─‹"         => MikaArm::GripperRight,
    "╮"          => MikaArm::Right,
    "⸍"          => MikaArm::RaisedRight,
    "⌐·"         => MikaArm::ShootRight,
    "◡-"         => MikaArm::ShrugRight,
    "ᗢ"          => MikaArm::Shield,
    "─"          => MikaArm::Point,
    "ᓀ"          => MikaArm::PunchRight,
    "ᓄ"          => MikaArm::PunchLowRight,
    "╯"          => MikaArm::UpRight,
    _ => unreachable!(),
  };
  Ok((input, arm))
}

// Expression Primitives
// ============================================================================

pub fn mika_eye_left(input: ParseString) -> ParseResult<Token> {
  alt((
    tag("⌐▰"),  // Shades — two chars, must precede single-char alternatives
    tag("ˆ"),  tag("ಠ"),  tag("╥"),  tag("⋇"),  tag("✖"),
    tag("≻"),  tag("ᗒ"),  tag("ㆆ"),  tag("◜"),  tag("˙"),
    tag("⚆"),  tag("☉"),  tag("◠"),  tag("◡̀"), tag("◕"),
    tag("◞"),  tag("Ͼ"),  tag("⹇"),  tag("≖"),  tag("°"),
    tag("¬"),  tag("◉"),  tag("ᗣ"),  tag("ᗩ"),
    // Mylo
    tag("ᑕ"),  tag("ᕮ"),  tag("ᕳ"),  tag("ᘭ"),  tag("ᑢ"),
  ))(input)
}

pub fn mika_eye_right(input: ParseString) -> ParseResult<Token> {
  alt((
    tag("▰"),
    tag("ˆ"),  tag("ಠ"),  tag("╥"),  tag("⋇"),  tag("✖"),
    tag("≺"),  tag("ᗕ"),  tag("ㆆ"),  tag("◝"),  tag("˙"),
    tag("⚆"),  tag("☉"),  tag("◠"),  tag("◡́"), tag("◕"),
    tag("◟"),  tag("Ͽ"),  tag("⹇"),  tag("≖"),  tag("°"),
    tag("¬"),  tag("◉"),  tag("ᗣ"),  tag("ᗩ"),
    // Mylo
    tag("ᑐ"),  tag("ᕭ"),  tag("ᕲ"),  tag("ᘪ"),  tag("ᑝ"),
  ))(input)
}

// mika-expression-inner := eye-left, nose, eye-right ;
// Matches on (left_str, MikaFace, right_str) — the nose is already a structured
// enum value from mika_nose, so we use it directly instead of re-stringifying.
// This means Glaring's ⍜ nose vs. everyone else's ◯ nose is handled by type,
// not by embedding magic strings in the match arms.
pub fn mika_expression_inner(input: ParseString) -> ParseResult<MikaExpression> {
  let (input, left_eye)  = mika_eye_left(input)?;
  let (input, nose_face) = mika_nose(input)?;
  let (input, right_eye) = mika_eye_right(input)?;

  let l = left_eye.chars.iter().collect::<String>();
  let r = right_eye.chars.iter().collect::<String>();

  let expr = match (l.as_str(), nose_face, r.as_str()) {
    ("ˆ",   MikaFace::Open,      "ˆ")   => MikaExpression::Content,
    ("ಠ",   MikaFace::Open,      "ಠ")   => MikaExpression::Confused,
    ("╥",   MikaFace::Open,      "╥")   => MikaExpression::Crying,
    ("⋇",   MikaFace::Open,      "⋇")   => MikaExpression::Dazed,
    ("✖",   MikaFace::Open,      "✖")   => MikaExpression::Dead,
    ("≻",   MikaFace::Open,      "≺")   => MikaExpression::EyesSqueezed,
    ("ᗒ",   MikaFace::Open,      "ᗕ")   => MikaExpression::SuperSqueezed,
    ("ㆆ",  MikaFace::FlatMouth, "ㆆ")  => MikaExpression::Glaring,
    ("◜",   MikaFace::Open,      "◝")   => MikaExpression::Happy,
    ("˙",   MikaFace::Open,      "˙")   => MikaExpression::Normal,
    ("⚆",   MikaFace::Open,      "⚆")   => MikaExpression::PeerRight,
    ("☉",   MikaFace::Open,      "☉")   => MikaExpression::PeerStraight,
    ("◠",   MikaFace::Open,      "◠")   => MikaExpression::Pleased,
    ("◡̀",  MikaFace::Open,      "◡́")  => MikaExpression::Resolved,
    ("◕",   MikaFace::Open,      "◕")   => MikaExpression::RollingEyes,
    ("◞",   MikaFace::Open,      "◟")   => MikaExpression::Sad,
    ("Ͼ",   MikaFace::Open,      "Ͽ")   => MikaExpression::Scared,
    ("⌐▰",  MikaFace::Open,      "▰")   => MikaExpression::Shades,
    ("⹇",   MikaFace::Open,      "⹇")   => MikaExpression::Sleeping,
    ("ᗣ",   MikaFace::Open,      "ᗣ")   => MikaExpression::Smiling,
    ("≖",   MikaFace::Open,      "≖")   => MikaExpression::Squinting,
    ("°",   MikaFace::Open,      "°")   => MikaExpression::Surprised,
    ("ᗩ",   MikaFace::Open,      "ᗩ")   => MikaExpression::TearingUp,
    ("¬",   MikaFace::Open,      "¬")   => MikaExpression::Unimpressed,
    ("◉",   MikaFace::Open,      "◉")   => MikaExpression::Wired,
    _ => {
      return Err(nom::Err::Error(ParseError {
        cause_range: SourceRange::default(),
        remaining_input: input,
        error_detail: ParseErrorDetail {
          message: "Unrecognized Mika expression",
          annotation_rngs: Vec::new(),
        },
      }));
    }
  };
  Ok((input, expr))
}

// Mika Character
// ============================================================================

// micro-mika := arm-left, face, arm-right ;   e.g. ╭⦿╮  ╰⦿╯  ─⦿╮  ╭⦿⌣
pub fn micro_mika(input: ParseString) -> ParseResult<Mika> {
  let (input, left_arm)  = mika_arm_left(input)?;
  let (input, _face)     = mika_nose(input)?;
  let (input, right_arm) = mika_arm_right(input)?;
  Ok((input, Mika::Micro(micromika_from_arms(left_arm, right_arm))))
}

// mini-mika := arm-left, "(", expression-inner, ")", arm-right ;   e.g. ╭(˙◯˙)╮
pub fn mini_mika(input: ParseString) -> ParseResult<Mika> {
  let (input, left_arm)   = mika_arm_left(input)?;
  let (input, _)          = left_parenthesis(input)?;
  let (input, expression) = mika_expression_inner(input)?;
  let (input, _)          = right_parenthesis(input)?;
  let (input, right_arm)  = mika_arm_right(input)?;
  Ok((input, Mika::Mini(MiniMika { expression, left_arm, right_arm })))
}

// mika := mini-mika | micro-mika ;
pub fn mika(input: ParseString) -> ParseResult<Mika> {
  alt((mini_mika, micro_mika))(input)
}

// Helpers
// ============================================================================

fn micromika_from_arms(left: MikaArm, right: MikaArm) -> MicroMika {
  use MikaArm::*;
  match (left, right) {
    (BatWing,      BatWing)       => MicroMika::Bat,
    (GestureLeft,  GestureRight)  => MicroMika::BigHug,
    (RaisedLeft,   RaisedRight)   => MicroMika::Cheer,
    (Dance,        Dance)         => MicroMika::Dance,
    (UpLeft,       UpRight)       => MicroMika::Goal,
    (GripperLeft,  UpRight)       => MicroMika::GripperLeft,
    (UpLeft,       GripperRight)  => MicroMika::GripperRight,
    (GestureLeft,  UpRight)       => MicroMika::GestureLeft,
    (UpLeft,       GestureRight)  => MicroMika::GestureRight,
    (Left,         Right)         => MicroMika::Idle,
    (Sword,        Shield)        => MicroMika::Knight,
    (ShootLeft,    ShootRight)    => MicroMika::Matrix,
    (Sword,        BatWing)       => MicroMika::OneWing,
    (Point,        UpRight)       => MicroMika::PointLeft,
    (UpLeft,       Point)         => MicroMika::PointRight,
    (PunchLeft,    PunchLowRight) => MicroMika::Punch,
    (ShootLeft,    UpRight)       => MicroMika::ShootLeft,
    (UpLeft,       ShootRight)    => MicroMika::ShootRight,
    (ShrugLeft,    ShrugRight)    => MicroMika::Shrug,
    (ShrugLeft,    UpRight)       => MicroMika::ServeLeft,
    (UpLeft,       ShrugRight)    => MicroMika::ServeRight,
    _                             => MicroMika::Idle,
  }
}