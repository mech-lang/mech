use crate::*;

// Mika
// ============================================================================

// Inline Mika
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MiniMIka {
  pub expression: MikaExpression,
  pub left_arm: MikaArm,
  pub right_arm: MikaArm,
}

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
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MicoMika {
  Bat,            // ᗑ⦿ᗑ
  BigHug,         // ›⌣⦿⌣‹
  Cheering,       // ⸌⦿⸍
  Dancing,        // ~⦿~
  Goal,           // ╰⦿╯
  GripperLeft,    // ›─⦿╮
  GripperRight,   // ╭⦿─‹
  GestureLeft,    // ⌣⦿╮
  GestureRight,   // ╭⦿⌣
  Idle,           // ╭⦿╮
  Knight,         // ⸸⦿ᗢ
  Matrix,         // ·¬⦿⌐·
  OneWing,        // ⸸⦿ᗑ
  PointingLeft,   // ╭⦿─
  PointingRight,  // ─⦿╮
  Punching,       // ᓂ⦿ᓄ
  ShootingLeft,   // ·¬⦿╮
  ShootingRight,  // ╭⦿⌐·
  Shrug,          // -◡⦿◡-
  ServingLeft,    // -◡⦿╮
  ServingRight,   // ╭⦿◡-
  WavingRight,    // ╭⦿╯
  WavingLeft,     // ╰⦿╮
}

impl MicoMika {
  pub fn symbols(&self) -> (MikaArm, MikaFace, MikaArm) {
    match self {
      MicoMika::Bat            => (MikaArm::Bat,          MikaFace::Normal, MikaArm::Bat),
      MicoMika::BigHug         => (MikaArm::GestureLeft,  MikaFace::Normal, MikaArm::GestureRight),
      MicoMika::Cheering       => (MikaArm::RaisedLeft,   MikaFace::Normal, MikaArm::RaisedRight),
      MicoMika::Dancing        => (MikaArm::Dance,        MikaFace::Normal, MikaArm::Dance),
      MicoMika::Goal           => (MikaArm::UpLeft,       MikaFace::Normal, MikaArm::UpRight),
      MicoMika::GripperLeft    => (MikaArm::GripperLeft,  MikaFace::Normal, MikaArm::UpRight),
      MicoMika::GripperRight   => (MikaArm::UpLeft,       MikaFace::Normal, MikaArm::GripperRight),
      MicoMika::GestureLeft    => (MikaArm::GestureLeft,  MikaFace::Normal, MikaArm::UpRight),
      MicoMika::GestureRight   => (MikaArm::UpLeft,       MikaFace::Normal, MikaArm::GestureRight),
      MicoMika::Idle           => (MikaArm::Left,         MikaFace::Normal, MikaArm::Right),
      MicoMika::Knight         => (MikaArm::Sword,        MikaFace::Normal, MikaArm::Shield),
      MicoMika::Matrix         => (MikaArm::ShootLeft,    MikaFace::Normal, MikaArm::ShootRight),
      MicoMika::OneWing        => (MikaArm::Sword,        MikaFace::Normal, MikaArm::Bat),
      MicoMika::PointingLeft   => (MikaArm::PointingLeft, MikaFace::Normal, MikaArm::UpRight),
      MicoMika::PointingRight  => (MikaArm::UpLeft,       MikaFace::Normal, MikaArm::PointingRight),
      MicoMika::Punching       => (MikaArm::PunchLeft,    MikaFace::Normal, MikaArm::PunchLowRight),
      MicoMika::ShootingLeft   => (MikaArm::ShootLeft,    MikaFace::Normal, MikaArm::UpRight),
      MicoMika::ShootingRight  => (MikaArm::UpLeft,       MikaFace::Normal, MikaArm::ShootRight),
      MicoMika::Shrug          => (MikaArm::ShrugLeft,    MikaFace::Normal, MikaArm::ShrugRight),
      MicoMika::ServingLeft    => (MikaArm::ServingLeft,  MikaFace::Normal, MikaArm::UpRight),
      MicoMika::ServingRight   => (MikaArm::UpLeft,       MikaFace::Normal, MikaArm::ServingRight),
      MicoMika::WavingLeft     => (MikaArm::UpLeft,       MikaFace::Normal, MikaArm::UpRight),
      MicoMika::WavingRight    => (MikaArm::UpLeft,       MikaFace::Normal, MikaArm::UpRight),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MikaArm {
  Bat,               // ᗑ
  BigGripperLeft,    // Ɔ∞
  BigGripperRight,   // ∞C
  CurlLeft,          // ᕦ
  CurlRight,         // ᕤ
  Dance,             // ~
  GesturingLeft,     // ›⌣
  GesturingRight,    // ⌣‹
  GripperLeft,       // ›─
  GripperRight,      // ─‹
  Left,              // ╭
  RaisedLeft,        // ⸌
  RaisedRight,       // ⸍
  Right,             // ╮
  Shield,            // ᗢ
  ShootingLeft,      // ·¬
  ShootingRight,     // ⌐·
  ShrugingLeft,      // -◡
  ShrugingRight,     // ◡-
  Sword,             // ⸸
  PointingLeft,      // ╭─
  PunchingLeft,      // ᓂ
  PunchingRight,     // ᓀ
  PunchingLowLeft,   // ᓇ
  PunchingLowRight,  // ᓄ
  UpLeft,            // ╰
  UpRight,           // ╯
}

impl MikaArm {
  pub fn symbol(&self) -> &'static str {
    match self {
      MikaArm::Bat => "ᗑ",
      MikaArm::BigGripperLeft => "Ɔ∞",
      MikaArm::BigGripperRight => "∞C",
      MikaArm::CurlLeft => "ᕦ",
      MikaArm::CurlRight => "ᕤ",
      MikaArm::Dance => "~",
      MikaArm::GesturingLeft => "⌣",
      MikaArm::GesturingRight => "⌣",
      MikaArm::GripperLeft => "›─",
      MikaArm::GripperRight => "─‹",
      MikaArm::Left => "╭",
      MikaArm::RaisedLeft => "⸌",
      MikaArm::RaisedRight => "⸍",
      MikaArm::Right => "╮",
      MikaArm::Shield => "ᗢ",
      MikaArm::ShootingLeft => "·¬",
      MikaArm::ShootingRight => "⌐·",
      MikaArm::ShrugingLeft => "-◡",
      MikaArm::ShrugingRight => "◡-",
      MikaArm::Sword => "⸸",
      MikaArm::PunchingLeft => "ᓂ",
      MikaArm::PunchingRight => "ᓀ",
      MikaArm::PunchingLowLeft => "ᓇ",
      MikaArm::PunchingLowRight => "ᓄ",
      MikaArm::UpLeft => "╰",
      MikaArm::UpRight => "╯",
    }
  }

  pub fn is_left(&self) -> bool {
    matches!(self, MikaArm::UpLeft | MikaArm::Bat | MikaArm::BigGripperLeft | MikaArm::CurlLeft | MikaArm::GesturingLeft | MikaArm::GripperLeft | MikaArm::Left | MikaArm::ShootingLeft | MikaArm::ShrugingLeft | MikaArm::Dance)
  }

  pub fn is_right(&self) -> bool {
    matches!(self, MikaArm::UpRight | MikaArm::Bat | MikaArm::BigGripperRight | MikaArm::CurlRight | MikaArm::GesturingRight | MikaArm::GripperRight | MikaArm::Right | MikaArm::ShootingRight | MikaArm::ShrugingRight | MikaArm::Dance)
  }

}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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