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

// Mika Section
// ---------------------------------------------------------------------------

// mika-speech-bubble := "⸢", +section-element, "⸥" ;
pub fn mika_section(input: ParseString) -> ParseResult<MikaSection> {
  let msg = "Expects ⸥ to close speech bubble";
  let (input, (_, r)) = range(mika_section_open)(input)?;
  let (input, elements) = many1(section_element)(input)?;
  let (input, _) = label!(mika_section_close, msg, r)(input)?;
  Ok((input, MikaSection { elements }))
}

// Face / Arm Primitives
// ---------------------------------------------------------------------------

// mika-face := "⦿" | "◯" | "⊕" | "∘" | "⦾" | "⊖" | "⦵" | "⊗" | "⏺" | "⍜" ;
pub fn mika_nose(input: ParseString) -> ParseResult<MikaNose> {
  let (input, tok) = alt((
    tag("⦿"), tag("◯"), tag("⊕"), tag("∘"),
    tag("⦾"), tag("⊖"), tag("⦵"), tag("⊗"),
    tag("⏺"), tag("⍜"), tag("⬢"), tag("⬟"), tag("⬣"), tag("⎔"),
  ))(input)?;
  let face = match tok.chars().collect::<String>().as_str() {
    "⦿" => MikaNose::Normal,
    "◯" => MikaNose::Open,
    "⊕" => MikaNose::Back,
    "∘" => MikaNose::Stage1,
    "⦾" => MikaNose::Stage2,
    "⊖" => MikaNose::Blink,
    "⦵" => MikaNose::Wide,
    "⊗" => MikaNose::Error,
    "⏺" => MikaNose::Filled,
    "⍜" => MikaNose::FlatMouth,
    "⬢" => MikaNose::Hexagon,
    "⬟" => MikaNose::Pentagon,
    "⬣" => MikaNose::Hexagon2,
    "⎔" => MikaNose::HexagonOpen,
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
/*
// Expression Primitives
// ---------------------------------------------------------------------------

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

// mika-expression-inner := "(", eye-left, nose, eye-right, ")" ;
pub fn mika_expression_inner(input: ParseString) -> ParseResult<MikaExpression> {
  let (input, left_eye)  = mika_eye_left(input)?;
  let (input, nose)      = mika_nose(input)?;
  let (input, right_eye) = mika_eye_right(input)?;
  let (input, _)         = right_parenthesis(input)?;

  let l = left_eye.chars.iter().collect::<String>();
  let n = nose.chars.iter().collect::<String>();
  let r = right_eye.chars.iter().collect::<String>();

  let expr = match (l.as_str(), n.as_str(), r.as_str()) {
    ("ˆ",   "◯", "ˆ")   => MikaExpression::Content,
    ("ಠ",   "◯", "ಠ")   => MikaExpression::Confused,
    ("╥",   "◯", "╥")   => MikaExpression::Crying,
    ("⋇",   "◯", "⋇")   => MikaExpression::Dazed,
    ("✖",   "◯", "✖")   => MikaExpression::Dead,
    ("≻",   "◯", "≺")   => MikaExpression::EyesSqueezed,
    ("ᗒ",   "◯", "ᗕ")   => MikaExpression::SuperSqueezed,
    ("ㆆ",  "⍜", "ㆆ")  => MikaExpression::Glaring,
    ("◜",   "◯", "◝")   => MikaExpression::Happy,
    ("˙",   "◯", "˙")   => MikaExpression::Normal,
    ("⚆",   "◯", "⚆")   => MikaExpression::PeerRight,
    ("☉",   "◯", "☉")   => MikaExpression::PeerStraight,
    ("◠",   "◯", "◠")   => MikaExpression::Pleased,
    ("◡̀",  "◯", "◡́")  => MikaExpression::Resolved,
    ("◕",   "◯", "◕")   => MikaExpression::RollingEyes,
    ("◞",   "◯", "◟")   => MikaExpression::Sad,
    ("Ͼ",   "◯", "Ͽ")   => MikaExpression::Scared,
    ("⌐▰",  "◯", "▰")   => MikaExpression::Shades,
    ("⹇",   "◯", "⹇")   => MikaExpression::Sleeping,
    ("ᗣ",   "◯", "ᗣ")   => MikaExpression::Smiling,
    ("≖",   "◯", "≖")   => MikaExpression::Squinting,
    ("°",   "◯", "°")   => MikaExpression::Surprised,
    ("ᗩ",   "◯", "ᗩ")   => MikaExpression::TearingUp,
    ("¬",   "◯", "¬")   => MikaExpression::Unimpressed,
    ("◉",   "◯", "◉")   => MikaExpression::Wired,
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
}*/

// Mika Character
// ---------------------------------------------------------------------------

// micro-mika := arm-left, face, arm-right ;   e.g. ╭⦿╮  ╰⦿╯  ─⦿╮  ╭⦿⌣
pub fn micro_mika(input: ParseString) -> ParseResult<Mika> {
  let (input, left_arm)  = mika_arm_left(input)?;
  let (input, right_arm) = mika_arm_right(input)?;
  Ok((input, Mika::Micro(micromika_from_arms(left_arm, right_arm))))
}
/*
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
}*/

// mika := mini-mika | micro-mika ;
pub fn mika(input: ParseString) -> ParseResult<(Mika,Option<MikaSection>)> {
  let (input, mika) = micro_mika(input)?;
  let (input, mika_section) = opt(mika_section)(input)?;
  Ok((input, (mika, mika_section)))
}

// Helpers
// ---------------------------------------------------------------------------

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