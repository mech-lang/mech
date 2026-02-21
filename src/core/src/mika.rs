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
  Bat,          // ᗑ⦿ᗑ
  BigHug,       // ›⌣⦿⌣‹
  Cheering,     // ╰⦿╯
  Dancing,      // ~⦿~
  GripperLeft,  // ›─⦿╮
  GripperRight, // ╭⦿─‹
  GestureLeft,  // ⌣⦿╮
  GestureRight, // ╭⦿⌣
  Idle,         // ╭⦿╮
  Knight,       // ⸸⦿ᗢ
  Matrix,       // ·¬⦿⌐·
  OWA,          // ⸸⦿ᗑ
  PointingLeft, // ╭⦿─
  PointingRight,// ─⦿╮
  Punching,     // ᓂ⦿ᓄ
  Shrug,        // -◡⦿◡-
  ServingLeft,  // -◡⦿╮
  ServingRight, // ╭⦿◡-
  WavingRight,  // ╭⦿╯
  WavingLeft,   // ╰⦿╮
}

impl SmallMika {
  pub fn symbols(&self) -> (MikaArm, MikaFace, MikaArm) {
    match self {
      SmallMika::Bat => (MikaArm::Bat, "⦿", MikaArm::Bat),
      SmallMika::BigHug => (MikaArm::GestureLeft, "⦿", MikaArm::GestureRight),
      SmallMika::Cheering => (MikaArm::UpLeft, "⦿", MikaArm::UpRight),
      SmallMika::Dancing => (MikaArm::Dance, "⦿", MikaArm::Dance),
      SmallMika::GripperLeft => (MikaArm::GripperLeft, "⦿", MikaArm::UpRight),
      SmallMika::GripperRight => (MikaArm::UpLeft, "⦿", MikaArm::GripperRight),
      SmallMika::Idle => (MikaArm::Left, "⦿", MikaArm::Right),
      SmallMika::Knight => (MikaArm::Sword, "⦿", MikaArm::Shield),
      SmallMika::Matrix => (MikaArm::ShootLeft, "⦿", MikaArm::ShootRight),
      SmallMika::PointingLeft => (MikaArm::PointingLeft, "⦿", MikaArm::UpRight),
      SmallMika::PointingRight => (MikaArm::UpLeft, "⦿", MikaArm::PointingRight),
      SmallMika::Shrug => (MikaArm::ShrugLeft, "⦿", MikaArm::ShrugRight),
      SmallMika::WavingLeft => (MikaArm::UpLeft, "⦿", MikaArm::UpRight),
      SmallMika::WavingRight => (MikaArm::UpLeft, "⦿", MikaArm::UpRight),
    }
  }
}

pub enum MikaArm {
  UpLeft,         // ╰
  UpRight,        // ╯
  Bat,            // ᗑ
  BigGripperLeft, // Ɔ∞
  BigGripperRight,// ∞C
  CurlLeft,       // ᕦ
  CurlRight,      // ᕤ
  Dance,          // ~
  GestureLeft,    // ›⌣
  GestureRight,   // ⌣‹
  GripperLeft,    // ›─
  GripperRight,   // ─‹
  Left,           // ╭
  Right,          // ╮
  Shield,         // ᗢ
  ShootLeft,      // ·¬
  ShootRight,     // ⌐·
  ShrugLeft,      // -◡
  ShrugRight,     // ◡-
  Sword,          // ⸸
  PunchLeft,      // ᓂ
  PunchRight,     // ᓀ
  PunchLowLeft,   // ᓇ
  PunchLowRight,  // ᓄ
}

impl MikaArm {
  pub fn symbol(&self) -> &'static str {
    match self {
      MikaArm::UpLeft => "╰",
      MikaArm::UpRight => "╯",
      MikaArm::Bat => "ᗑ",
      MikaArm::BigGripperLeft => "Ɔ∞",
      MikaArm::BigGripperRight => "∞C",
      MikaArm::CurlLeft => "ᕦ",
      MikaArm::CurlRight => "ᕤ",
      MikaArm::Dance => "~",
      MikaArm::GestureLeft => "›⌣",
      MikaArm::GestureRight => "⌣‹",
      MikaArm::GripperLeft => "›─",
      MikaArm::GripperRight => "─‹",
      MikaArm::Left => "╭",
      MikaArm::Right => "╮",
      MikaArm::Shield => "ᗢ",
      MikaArm::ShootLeft => "·¬",
      MikaArm::ShootRight => "⌐·",
      MikaArm::ShrugLeft => "-◡",
      MikaArm::ShrugRight => "◡-",
      MikaArm::Sword => "⸸",
      MikaArm::PunchLeft => "ᓂ",
      MikaArm::PunchRight => "ᓀ",
      MikaArm::PunchLowLeft => "ᓇ",
      MikaArm::PunchLowRight => "ᓄ",
    }
  }

  pub fn is_left(&self) -> bool {
    matches!(self, MikaArm::UpLeft | MikaArm::Bat | MikaArm::BigGripperLeft | MikaArm::CurlLeft | MikaArm::GestureLeft | MikaArm::GripperLeft | MikaArm::Left | MikaArm::ShootLeft | MikaArm::ShrugLeft | MikaArm::Dance)
  }

  pub fn is_right(&self) -> bool {
    matches!(self, MikaArm::UpRight | MikaArm::Bat | MikaArm::BigGripperRight | MikaArm::CurlRight | MikaArm::GestureRight | MikaArm::GripperRight | MikaArm::Right | MikaArm::ShootRight | MikaArm::ShrugRight | MikaArm::Dance)
  }

}

// Mylo is a secondary character, he's under development right now on the basis of these faces. Maybe he's a villain? Maybe he's Mika's siblng? I don't know.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MyloExpression {
  Eyes,       // (ᑕ◯ᑐ)
  Focused,    // (ᕮ◯ᕭ)
  Alarm,      // (ᕳ◯ᕲ)
  Angry,      // (ᘭ◯ᘪ)
  Crossed,    // (ᑢ◯ᑝ)
}