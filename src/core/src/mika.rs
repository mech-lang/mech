use crate::*;

// Mika
// ============================================================================

// Inline Mika
// -----------------------------------------------------------------------------

// Inline Mika lives in the terminal. She greets users when they start Mech, and provides a friendly face to interact with. She can display a variety of expressions and poses, and can be used to add personality and fun to the Mech experience. Users can customize Mika's appearance and expressions, and she can be used to provide feedback, celebrate achievements, or just add a bit of whimsy to the coding process.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MikaExpression {
  Content,       // (ˆ◯ˆ)
  Confused,      // (ಠ◯ಠ) 
  Crying,        // (╥◯╥)
  Dazed,         // (⋇◯⋇)
  Dead,          // (✖◯✖)
  EyesSqueezed,  // (≻◯≺)
  SuperSqueezed, // (ᗒ◯ᗕ)
  Glaring,       // (ㆆ⍜ㆆ)
  Happy,         // (◜◯◝)
  Normal,        // (˙◯˙)
  PeerRight,     // (⚆◯⚆)
  PeerStraight,  // (☉◯☉)
  Pleased,       // (◠◯◠)
  Resolved,      // (◡̀◯◡́)ᕤ
  RollingEyes,   // (◕◯◕)
  Sad,           // (◞◯◟)
  Scared,        // (Ͼ◯Ͽ)
  Shades,        // (⌐▰◯▰)
  Sleeping,      // (-◯-) 「Hello Welcome to Mech! I'm Mika!」
  Smiling,       // (ᗣ◯ᗣ)
  Squinting,     // (≖◯≖)
  Surprised,     // (°◯°)
  TearingUp,     // (ᗩ◯ᗩ)
  Wired,         // (◉◯◉)
}

impl MikaExpression {
  pub fn symbols(&self) -> (&'static str, &'static str, &'static str) {
    match self {
      MikaExpression::Content => ("ˆ", "◯", "ˆ"),
      MikaExpression::Confused => ("ಠ", "◯", "ಠ"),
      MikaExpression::Crying => ("╥", "◯", "╥"),
      MikaExpression::Dazed => ("⋇", "◯", "⋇"),
      MikaExpression::Dead => ("✖", "◯", "✖"),
      MikaExpression::EyesSqueezed => ("≻", "◯", "≺"),
      MikaExpression::Glaring => ("ㆆ", "⍜", "ㆆ"),
      MikaExpression::Happy => ("◜", "◯", "◝"),
      MikaExpression::Normal => ("˙", "◯", "˙"),
      MikaExpression::PeerRight => ("⚆", "◯", "⚆"),
      MikaExpression::PeerStraight => ("☉", "◯", "☉"),
      MikaExpression::Pleased => ("◠", "◯", "◠"),
      MikaExpression::Resolved => ("◡̀", "◯", "◡́"),
      MikaExpression::RollingEyes => ("◕", "◯", "◕"),
      MikaExpression::Sad => ("◞", "◯", "◟"),
      MikaExpression::Scared => ("Ͼ", "◯", "Ͽ"),
      MikaExpression::Shades => ("⌐▰", "◯", "▰"),
      MikaExpression::Sleeping => ("-", "◯", "-"),
      MikaExpression::Squinting => ("≖", "◯", "≖"),
      MikaExpression::Surprised => ("°", "◯", "°"),
      MikaExpression::Wired => ("◉", "◯", "◉"),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SmallMika {
  Bat,         // ᗑ⦿ᗑ
  Idle,        // ╭⦿╮
  Pointing,    // ╭⦿─
  Waving,      // ╭⦿╯
  Cheering,    // ╰⦿╯
  Shrug,       // -◡⦿◡-
  Gripper,     // ╭⦿─‹
  BigHug,      // ›⌣⦿⌣‹
  Knight,      // ⸸⦿ᗢ
  Matrix,      // ·¬⦿⌐·
  Dancing,     // ~⦿~
}

impl SmallMika {
  pub fn symbols(&self) -> (&'static str, &'static str, &'static str) {
    match self {
      SmallMika::Bat => ("ᗑ", "⦿", "ᗑ"),
      SmallMika::BigHug => ("›⌣", "⦿", "⌣‹"),
      SmallMika::Cheering => ("╰", "⦿", "╯"),
      SmallMika::Dancing => ("~", "⦿", "~"),
      SmallMika::GripperLeft => ("›─", "⦿", "╮"),
      SmallMika::GripperRight => ("╭", "⦿", "─‹"),
      SmallMika::Idle => ("╭", "⦿", "╮"),
      SmallMika::Knight => ("⸸", "⦿", "ᗢ"),
      SmallMika::Matrix => ("·¬", "⦿", "⌐·"),
      SmallMika::PointingLeft => ("─", "⦿", "╮"),
      SmallMika::PointingRight => ("╭", "⦿", "─"),
      SmallMika::Shrug => ("-◡", "⦿", "◡-"),
      SmallMika::WavingLeft => ("╰", "⦿", "╮"),
      SmallMika::WavingRight => ("╭", "⦿", "╯"),
    }
  }
}

pub enum MikaArm {
  CurlRight    // ᕤ
  CurlLeft     // ᕦ
  GripperRight // ∞C
  GripperLeft  // Ɔ∞
}


// Mylo is a secondary character, he's under development right no on the basis of these facses. Maybe he's a villain? Maybe he's Mika's siblng.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MyloExpression {
  Eyes,       // (ᑕ◯ᑐ)
  Focused,    // (ᕮ◯ᕭ)
  Alarm,      // (ᕳ◯ᕲ)
  Angry,      // (ᘭ◯ᘪ)
  Crossed,    // (ᑢ◯ᑝ)
}