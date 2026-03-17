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

pub static MICROMIKA_WAVE: &[&str] = &[
  "╭◉╮", "╭◉─", "╭◉╯", "╭◉─", "╭◉╯", "╭◉─", "╭◉╮", " "
];

pub static MICROMIKA_SLEEP: &[&str] = &[
  "╭◉╮", "╭⦾╮", "╭⊚╮", "╭⊙╮", "╭◯╮"
];

pub static MICROMIKA_WAKE: &[&str] = &[
  "╭◯╮", "╭⊙╮", "╭⊚╮", "╭⦾╮", "╭◉╮"
];

pub static MICROMIKA_BLINK: &[&str] = &[
  "╭◉╮", "╭⊖╮", "╭◉╮", "╭⊖╮", "╭◉╮", "╭◉╮", "╭◉╮", "╭◉╮"
];

pub static MICROMIKA_PULSE: &[&str] = &[
  "╭◉╮", "╭⦾╮", "╭⊚╮", "╭⊙╮", "╭⊚╮", "╭⦾╮", "╭◉╮"
];

pub static MICROMIKA_RAISE: &[&str] = &[
  "╭◉╮", "─◉─", "╰◉╯"
];

pub static MICROMIKA_FLAP: &[&str] = &[
  "╭◉╮", "─◉─", "╰◉╯", "─◉─", "╭◉╮"
];

pub static MICROMIKA_ATTENTION: &[&str] = &[
  "╭◉╯", "╭◉╯", "╭◉╯","╭◉╯","╭◉╯", "╭◉─", "╭◉╯", "╭◉─",
];

// Mika Section
// ---------------------------------------------------------------------------

// mika-speech-bubble := "⸢", +section-element, "⸥" ;
pub fn mika_section(input: ParseString) -> ParseResult<MikaSection> {
  let msg = "Expects ⸥ to close speech bubble";
  let (input, (_, r)) = range(mika_section_open)(input)?;
  let (input, elements) = section(input)?;
  let (input, _) = label!(mika_section_close, msg, r)(input)?;
  Ok((input, MikaSection { elements }))
}

// Face / Arm Primitives
// ---------------------------------------------------------------------------

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
    tag("~"),  // Dance
    tag("⌣"),  // GestureLeft (bare)
    tag("╭"),  // Left
    tag("⸌"),  // RaisedLeft
    tag("⸸"),  // Sword
    tag("─"),  // Point
    tag("ᓂ"),  // PunchLeft
    tag("ᓇ"),  // PunchLowLeft
    tag("╰"),  // UpLeft
  ))(input)?;
  let arm = match tok.chars().collect::<String>().as_str() {
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
  let arm = match tok.chars().collect::<String>().as_str() {
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
// ---------------------------------------------------------------------------

// The order matters — longer tags must be tried before any of their prefixes.
// MikaEyeLeft::Shades is "⌐▰" (two chars) and must precede all single-char tags.
const LEFT_EYE_ORDER: &[MikaEyeLeft] = &[
  MikaEyeLeft::Shades,        // "⌐▰"  — multi-char, must be first
  MikaEyeLeft::Content,       // "ˆ"
  MikaEyeLeft::Confused,      // "ಠ"
  MikaEyeLeft::Crying,        // "╥"
  MikaEyeLeft::Dazed,         // "⋇"
  MikaEyeLeft::Dead,          // "✖"
  MikaEyeLeft::EyesSqueezed,  // "≻"
  MikaEyeLeft::SuperSqueezed, // "ᗒ"
  MikaEyeLeft::Glaring,       // "ㆆ"
  MikaEyeLeft::Happy,         // "◜"
  MikaEyeLeft::Normal,        // "˙"
  MikaEyeLeft::PeerRight,     // "⚆"
  MikaEyeLeft::PeerStraight,  // "☉"
  MikaEyeLeft::Pleased,       // "◠"
  MikaEyeLeft::Resolved,      // "◡̀"
  MikaEyeLeft::RollingEyes,   // "◕"
  MikaEyeLeft::Sad,            // "◞"
  MikaEyeLeft::Scared,        // "Ͼ"
  MikaEyeLeft::Sleeping,      // "⹇"
  MikaEyeLeft::Smiling,       // "ᗣ"
  MikaEyeLeft::Squinting,     // "≖"
  MikaEyeLeft::Surprised,     // "°"
  MikaEyeLeft::TearingUp,     // "ᗩ"
  MikaEyeLeft::Unimpressed,   // "¬"
  MikaEyeLeft::Wired,         // "◉"
];

const RIGHT_EYE_ORDER: &[MikaEyeRight] = &[
  MikaEyeRight::Content,
  MikaEyeRight::Confused,
  MikaEyeRight::Crying,
  MikaEyeRight::Dazed,
  MikaEyeRight::Dead,
  MikaEyeRight::EyesSqueezed,
  MikaEyeRight::SuperSqueezed,
  MikaEyeRight::Glaring,
  MikaEyeRight::Happy,
  MikaEyeRight::Normal,
  MikaEyeRight::PeerRight,
  MikaEyeRight::PeerStraight,
  MikaEyeRight::Pleased,
  MikaEyeRight::Resolved,
  MikaEyeRight::RollingEyes,
  MikaEyeRight::Sad,
  MikaEyeRight::Scared,
  MikaEyeRight::Shades,       // "▰"
  MikaEyeRight::Sleeping,
  MikaEyeRight::Smiling,
  MikaEyeRight::Squinting,
  MikaEyeRight::Surprised,
  MikaEyeRight::TearingUp,
  MikaEyeRight::Unimpressed,
  MikaEyeRight::Wired,
];

const NOSE_ORDER: &[MikaNose] = &[
  MikaNose::Normal,
  MikaNose::Open,
  MikaNose::Back,
  MikaNose::Stage1,
  MikaNose::Stage2,
  MikaNose::Stage3,
  MikaNose::Blink,
  MikaNose::Wide,
  MikaNose::Error,
  MikaNose::Filled,
  MikaNose::FlatMouth,
  MikaNose::Hexagon,
  MikaNose::Pentagon,
  MikaNose::Hexagon2,
  MikaNose::HexagonOpen,
];

// All known Mika expressions — used by mika_expression_inner to resolve
// (left_eye, nose) -> MikaExpression and then consume the expected right eye.
const EXPRESSIONS: &[MikaExpression] = &[
  MikaExpression::Content,
  MikaExpression::Confused,
  MikaExpression::Crying,
  MikaExpression::Dazed,
  MikaExpression::Dead,
  MikaExpression::EyesSqueezed,
  MikaExpression::SuperSqueezed,
  MikaExpression::Glaring,
  MikaExpression::Happy,
  MikaExpression::Normal,
  MikaExpression::PeerRight,
  MikaExpression::PeerStraight,
  MikaExpression::Pleased,
  MikaExpression::Resolved,
  MikaExpression::RollingEyes,
  MikaExpression::Sad,
  MikaExpression::Scared,
  MikaExpression::Shades,
  MikaExpression::Sleeping,
  MikaExpression::Smiling,
  MikaExpression::Squinting,
  MikaExpression::Surprised,
  MikaExpression::TearingUp,
  MikaExpression::Unimpressed,
  MikaExpression::Wired,
];

pub fn mika_eye_left(input: ParseString) -> ParseResult<MikaEyeLeft> {
  for &variant in LEFT_EYE_ORDER {
    if let Ok((rest, _)) = tag(variant.symbol())(input.clone()) {
      return Ok((rest, variant));
    }
  }
  Err(nom::Err::Error(ParseError {
    cause_range: SourceRange::default(),
    remaining_input: input,
    error_detail: ParseErrorDetail {
      message: "Expected Mika left eye",
      annotation_rngs: Vec::new(),
    },
  }))
}

pub fn mika_eye_right(input: ParseString) -> ParseResult<MikaEyeRight> {
  for &variant in RIGHT_EYE_ORDER {
    if let Ok((rest, _)) = tag(variant.symbol())(input.clone()) {
      return Ok((rest, variant));
    }
  }
  Err(nom::Err::Error(ParseError {
    cause_range: SourceRange::default(),
    remaining_input: input,
    error_detail: ParseErrorDetail {
      message: "Expected Mika right eye",
      annotation_rngs: Vec::new(),
    },
  }))
}

// mika-nose := "⦿" | "◯" | "⊕" | "∘" | "⦾" | "⊖" | "⦵" | "⊗" | "⏺" | "⍜" ;
pub fn mika_nose(input: ParseString) -> ParseResult<MikaNose> {
  for &variant in NOSE_ORDER {
    if let Ok((rest, _)) = tag(variant.symbol())(input.clone()) {
      return Ok((rest, variant));
    }
  }
  Err(nom::Err::Error(ParseError {
    cause_range: SourceRange::default(),
    remaining_input: input,
    error_detail: ParseErrorDetail {
      message: "Expected Mika nose",
      annotation_rngs: Vec::new(),
    },
  }))
}

// mika-expression-inner := eye-left, nose, eye-right;
pub fn mika_expression_inner(input: ParseString) -> ParseResult<MikaExpression> {
  let (input, left) = mika_eye_left(input)?;
  let (input, nose) = mika_nose(input)?;

  let expr = EXPRESSIONS.iter().find(|&&e| {
    let (l, n, _) = e.symbols();
    l == left && n == nose
  });

  match expr {
    Some(&expr) => {
      let (_, _, right) = expr.symbols();
      let (input, _) = tag(right.symbol())(input)?;
      Ok((input, expr))
    }
    None => Err(nom::Err::Error(ParseError {
      cause_range: SourceRange::default(),
      remaining_input: input,
      error_detail: ParseErrorDetail {
        message: "Unrecognized Mika expression",
        annotation_rngs: Vec::new(),
      },
    })),
  }
}

// Mika Character
// ---------------------------------------------------------------------------

// micro-mika := arm-left, face, arm-right ;   e.g. ╭⦿╮  ╰⦿╯  ─⦿╮  ╭⦿⌣
pub fn micro_mika(input: ParseString) -> ParseResult<Mika> {
  let (input, left_arm)  = mika_arm_left(input)?;
  let (input, nose)      = mika_nose(input)?;
  let (input, right_arm) = mika_arm_right(input)?;
  Ok((input, Mika::Micro(MicroMika{ left_arm, nose, right_arm})))
}

// mini-mika := arm-left, expression-inner, arm-right ;   e.g. ╭(˙◯˙)╮
pub fn mini_mika(input: ParseString) -> ParseResult<Mika> {
  let (input, left_arm)        = opt(mika_arm_left)(input)?;
  let (input, _)               = left_parenthesis(input)?;
  let (input, right_arm_left)  = opt(mika_arm_right)(input)?;
  let (input, expression)      = mika_expression_inner(input)?;
  let (input, left_arm_right)  = opt(mika_arm_left)(input)?;
  let (input, _)               = right_parenthesis(input)?;
  let (input, right_arm)       = opt(mika_arm_right)(input)?;
  Ok((input, Mika::Mini(MiniMika { expression, left_arm: left_arm.or(right_arm_left), right_arm: right_arm.or(left_arm_right) })))
}

// mika := mini-mika | micro-mika ;
pub fn mika(input: ParseString) -> ParseResult<(Mika,Option<MikaSection>)> {
  let (input, mika) = alt((mini_mika, micro_mika))(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, mika_section) = opt(mika_section)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, (mika, mika_section)))
}