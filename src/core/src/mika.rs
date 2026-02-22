use crate::*;

// Mika
// ============================================================================

// Inline Mika
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mika {
  Mini(MiniMika),
  Micro(MicroMika)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MiniMika {
  pub expression: MikaExpression,
  pub left_arm: MikaArm,
  pub right_arm: MikaArm,
}

// Inline Mika lives in the terminal. She greets users when they start Mech, and provides a friendly face to interact with. She can display a variety of expressions and poses, and can be used to add personality and fun to the Mech experience. Users can customize Mika's appearance and expressions, and she can be used to provide feedback, celebrate achievements, or just add a bit of whimsy to the coding process.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
  Sleeping,      // (⹇◯⹇)
  Smiling,       // (ᗣ◯ᗣ)
  Squinting,     // (≖◯≖)
  Surprised,     // (°◯°)
  TearingUp,     // (ᗩ◯ᗩ)
  Unimpressed,   // (¬◯¬)
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
      MikaExpression::Sleeping => ("⹇", "◯", "⹇"),
      MikaExpression::Squinting => ("≖", "◯", "≖"),
      MikaExpression::Surprised => ("°", "◯", "°"),
      MikaExpression::Unimpressed => ("¬", "◯", "¬"),
      MikaExpression::Wired => ("◉", "◯", "◉"),
      MikaExpression::Smiling => ("ᗣ", "◯", "ᗣ"),
      MikaExpression::SuperSqueezed => ("ᗒ", "◯", "ᗕ"),
      MikaExpression::TearingUp => ("ᗩ", "◯", "ᗩ"),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MicroMika {
  Bat,            // ᗑ⦿ᗑ
  BigHug,         // ›⌣⦿⌣‹
  Cheer,          // ⸌⦿⸍
  Dance,          // ~⦿~
  Goal,           // ╰⦿╯
  GripperLeft,    // ›─⦿╮
  GripperRight,   // ╭⦿─‹
  GestureLeft,    // ⌣⦿╮
  GestureRight,   // ╭⦿⌣
  Idle,           // ╭⦿╮
  Knight,         // ⸸⦿ᗢ
  Matrix,         // ·¬⦿⌐·
  OneWing,        // ⸸⦿ᗑ
  PointLeft,      // ╭⦿─
  PointRight,     // ─⦿╮
  Punch,          // ᓂ⦿ᓄ
  ShootLeft,      // ·¬⦿╮
  ShootRight,     // ╭⦿⌐·
  Shrug,          // -◡⦿◡-
  ServeLeft,      // -◡⦿╮
  ServeRight,     // ╭⦿◡-
  WaveLeft,       // ╰⦿╮
  WaveRight,      // ╭⦿╯
}

impl MicroMika {
  pub fn symbols(&self) -> (MikaArm, MikaFace, MikaArm) {
    match self {
      MicroMika::Bat            => (MikaArm::BatWing,     MikaFace::Normal,  MikaArm::BatWing),
      MicroMika::BigHug         => (MikaArm::GestureLeft, MikaFace::Normal,  MikaArm::GestureRight),
      MicroMika::Cheer          => (MikaArm::RaisedLeft,  MikaFace::Normal,  MikaArm::RaisedRight),
      MicroMika::Dance          => (MikaArm::Dance,       MikaFace::Normal,  MikaArm::Dance),
      MicroMika::Goal           => (MikaArm::UpLeft,      MikaFace::Normal,  MikaArm::UpRight),
      MicroMika::GripperLeft    => (MikaArm::GripperLeft, MikaFace::Normal,  MikaArm::UpRight),
      MicroMika::GripperRight   => (MikaArm::UpLeft,      MikaFace::Normal,  MikaArm::GripperRight),
      MicroMika::GestureLeft    => (MikaArm::GestureLeft, MikaFace::Normal,  MikaArm::UpRight),
      MicroMika::GestureRight   => (MikaArm::UpLeft,      MikaFace::Normal,  MikaArm::GestureRight),
      MicroMika::Idle           => (MikaArm::Left,        MikaFace::Normal,  MikaArm::Right),
      MicroMika::Knight         => (MikaArm::Sword,       MikaFace::Normal,  MikaArm::Shield),
      MicroMika::Matrix         => (MikaArm::ShootLeft,   MikaFace::Normal,  MikaArm::ShootRight),
      MicroMika::OneWing        => (MikaArm::Sword,       MikaFace::Normal,  MikaArm::BatWing),
      MicroMika::PointLeft      => (MikaArm::Point,       MikaFace::Normal,  MikaArm::UpRight),
      MicroMika::PointRight     => (MikaArm::UpLeft,      MikaFace::Normal,  MikaArm::Point),
      MicroMika::Punch          => (MikaArm::PunchLeft,   MikaFace::Normal,  MikaArm::PunchLowRight),
      MicroMika::ShootLeft      => (MikaArm::ShootLeft,   MikaFace::Normal,  MikaArm::UpRight),
      MicroMika::ShootRight     => (MikaArm::UpLeft,      MikaFace::Normal,  MikaArm::ShootRight),
      MicroMika::Shrug          => (MikaArm::ShrugLeft,   MikaFace::Normal,  MikaArm::ShrugRight),
      MicroMika::ServeLeft      => (MikaArm::ShrugLeft,   MikaFace::Normal,  MikaArm::UpRight),
      MicroMika::ServeRight     => (MikaArm::UpLeft,      MikaFace::Normal,  MikaArm::ShrugRight),
      MicroMika::WaveLeft       => (MikaArm::UpLeft,      MikaFace::Normal,  MikaArm::UpRight),
      MicroMika::WaveRight      => (MikaArm::UpLeft,      MikaFace::Normal,  MikaArm::UpRight),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MikaArm {
  BatWing,           // ᗑ
  BigGripperLeft,    // Ɔ∞
  BigGripperRight,   // ∞C
  CurlLeft,          // ᕦ
  CurlRight,         // ᕤ
  Dance,             // ~
  GestureLeft,        // ›⌣
  GestureRight,       // ⌣‹
  GripperLeft,       // ›─
  GripperRight,      // ─‹
  Left,              // ╭
  RaisedLeft,        // ⸌
  RaisedRight,       // ⸍
  Right,             // ╮
  Shield,            // ᗢ
  ShootLeft,         // ·¬
  ShootRight,        // ⌐·
  ShrugLeft,        // -◡
  ShrugRight,       // ◡-
  Sword,             // ⸸
  Point,             // ─
  PunchLeft,         // ᓂ
  PunchRight,        // ᓀ
  PunchLowLeft,      // ᓇ
  PunchLowRight,     // ᓄ
  UpLeft,            // ╰
  UpRight,           // ╯
}

impl MikaArm {
  pub fn symbol(&self) -> &'static str {
    match self {
      MikaArm::BatWing => "ᗑ",
      MikaArm::BigGripperLeft => "Ɔ∞",
      MikaArm::BigGripperRight => "∞C",
      MikaArm::CurlLeft => "ᕦ",
      MikaArm::CurlRight => "ᕤ",
      MikaArm::Dance => "~",
      MikaArm::GestureLeft => "⌣",
      MikaArm::GestureRight => "⌣",
      MikaArm::GripperLeft => "›─",
      MikaArm::GripperRight => "─‹",
      MikaArm::Left => "╭",
      MikaArm::RaisedLeft => "⸌",
      MikaArm::RaisedRight => "⸍",
      MikaArm::Right => "╮",
      MikaArm::Shield => "ᗢ",
      MikaArm::ShootLeft => "·¬",
      MikaArm::ShootRight => "⌐·",
      MikaArm::ShrugLeft => "-◡",
      MikaArm::ShrugRight => "◡-",
      MikaArm::Sword => "⸸",
      MikaArm::Point => "─",
      MikaArm::PunchLeft => "ᓂ",
      MikaArm::PunchRight => "ᓀ",
      MikaArm::PunchLowLeft => "ᓇ",
      MikaArm::PunchLowRight => "ᓄ",
      MikaArm::UpLeft => "╰",
      MikaArm::UpRight => "╯",
    }
  }

  pub fn is_left(&self) -> bool {
    matches!(self, MikaArm::UpLeft | MikaArm::BatWing | MikaArm::BigGripperLeft | MikaArm::CurlLeft | MikaArm::GestureLeft | MikaArm::GripperLeft | MikaArm::Left | MikaArm::ShootLeft | MikaArm::ShrugLeft | MikaArm::Dance)
  }

  pub fn is_right(&self) -> bool {
    matches!(self, MikaArm::UpRight | MikaArm::BatWing | MikaArm::BigGripperRight | MikaArm::CurlRight | MikaArm::GestureRight | MikaArm::GripperRight | MikaArm::Right | MikaArm::ShootRight | MikaArm::ShrugRight | MikaArm::Dance)
  }

}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MikaFace {
  Normal,      // ⦿
  Open,        // ◯
  Back,        // ⊕
  Stage1,      // ⊙
  Stage2,      // ⊚
  Stage3,      // ⦾
  Blink,       // ⊖
  Wide,        // ⦵
  Error,       // ⊗
  Filled,      // ⏺
  FlatMouth,   // ⍜
}

impl MikaFace {
  pub fn symbol(&self) -> &'static str {
    match self {
      MikaFace::Normal => "⦿",
      MikaFace::Open => "◯",
      MikaFace::Back => "⊕",
      MikaFace::Stage1 => "∘",
      MikaFace::Stage2 => "⦾",
      MikaFace::Stage3 => "⦾",
      MikaFace::Blink => "⊖",
      MikaFace::Wide => "⦵",
      MikaFace::Error => "⊗",
      MikaFace::Filled => "⏺",
      MikaFace::FlatMouth => "⍜",
    }
  }
}

// Mylo is a secondary character, he's under development right now on the basis of these faces. Maybe he's a villain? Maybe he's Mika's siblng? I don't know.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MyloExpression {
  Eyes,       // (ᑕ◯ᑐ)
  Focused,    // (ᕮ◯ᕭ)
  Alarm,      // (ᕳ◯ᕲ)
  Angry,      // (ᘭ◯ᘪ)
  Crossed,    // (ᑢ◯ᑝ)
}

impl MyloExpression {
  pub fn symbols(&self) -> (&'static str, &'static str, &'static str) {
    match self {
      MyloExpression::Eyes => ("ᑕ", "◯", "ᑐ"),
      MyloExpression::Focused => ("ᕮ", "◯", "ᕭ"),
      MyloExpression::Alarm => ("ᕳ", "◯", "ᕲ"),
      MyloExpression::Angry => ("ᘭ", "◯", "ᘪ"),
      MyloExpression::Crossed => ("ᑢ", "◯", "ᑝ"),
    }
  }
}