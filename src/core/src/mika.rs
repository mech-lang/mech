use crate::*;

// Mika
// ============================================================================

// Inline Mika lives in the terminal. She greets users when they start Mech, and provides a friendly face to interact with. She can be depicted in a variety of expressions, sizes, colors, and poses. 

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MikaSection {
  pub elements: Vec<SectionElement>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mika {
  Mini(MiniMika),
  Micro(MicroMika),
}

impl Mika {

  pub fn to_string(&self) -> String {
    match self {
      Mika::Mini(mini) => mini.to_string(),
      Mika::Micro(micro) => micro.to_string(),
    }
  }

  pub fn tokens(&self) -> Vec<Token> {
    todo!();
  }
}

#[derive(Clone, Copy,Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MiniMika {
  pub expression: MikaExpression,
  pub left_arm: Option<MikaArm>,
  pub right_arm: Option<MikaArm>,
}

impl MiniMika {
  pub fn to_string(&self) -> String {
    let (left_eye, nose, right_eye) = self.expression.symbols();
    let left_arm = self.left_arm.map_or("", |arm| arm.symbol());
    let right_arm = self.right_arm.map_or("", |arm| arm.symbol());
    format!("{}({}{}{}){}", left_arm, left_eye, nose, right_eye, right_arm)
  }
}

// Parts
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MikaNose {
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
  Hexagon,     // ⬢
  Pentagon,    // ⬟
  Hexagon2,    // ⬣
  HexagonOpen, // ⎔
}

impl std::fmt::Display for MikaNose {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.symbol())
  }
}

impl MikaNose {
  pub fn symbol(&self) -> &'static str {
    match self {
      MikaNose::Normal => "⦿",
      MikaNose::Open => "◯",
      MikaNose::Back => "⊕",
      MikaNose::Stage1 => "∘",
      MikaNose::Stage2 => "⦾",
      MikaNose::Stage3 => "⦾",
      MikaNose::Blink => "⊖",
      MikaNose::Wide => "⦵",
      MikaNose::Error => "⊗",
      MikaNose::Filled => "⏺",
      MikaNose::FlatMouth => "⍜",
      MikaNose::Hexagon => "⬢",
      MikaNose::Pentagon => "⬟",
      MikaNose::Hexagon2 => "⬣",
      MikaNose::HexagonOpen => "⎔",
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
  GestureLeft,       // ›⌣
  GestureRight,      // ⌣‹
  GripperLeft,       // ›─
  GripperRight,      // ─‹
  Left,              // ╭
  RaisedLeft,        // ⸌
  RaisedRight,       // ⸍
  Right,             // ╮
  Shield,            // ᗢ
  ShootLeft,         // ·¬
  ShootRight,        // ⌐·
  ShrugLeft,         // -◡
  ShrugRight,        // ◡-
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
pub enum MikaEyeLeft {
  Content,       // ˆ
  Confused,      // ಠ 
  Crying,        // ╥
  Dazed,         // ⋇
  Dead,          // ✖
  EyesSqueezed,  // ≻
  SuperSqueezed, // ᗒ
  Glaring,       // ㆆ
  Happy,         // ◜
  Normal,        // ˙
  PeerRight,     // ⚆
  PeerStraight,  // ☉
  Pleased,       // ◠
  Resolved,      // ◡̀
  RollingEyes,   // ◕
  Sad,           // ◞
  Scared,        // Ͼ
  Shades,        // ⌐▰
  Sleeping,      // ⹇
  Smiling,       // ᗣ
  Squinting,     // ≖
  Surprised,     // °
  TearingUp,     // ᗩ
  Unimpressed,   // ¬
  Wired,         // ◉
}

impl Display for MikaEyeLeft {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.symbol())
  }
}

impl MikaEyeLeft {
  pub fn symbol(&self) -> &'static str {
    match self {
      MikaEyeLeft::Content => "ˆ",
      MikaEyeLeft::Confused => "ಠ",
      MikaEyeLeft::Crying => "╥",
      MikaEyeLeft::Dazed => "⋇",
      MikaEyeLeft::Dead => "✖",
      MikaEyeLeft::EyesSqueezed => "≻",
      MikaEyeLeft::SuperSqueezed => "ᗒ",
      MikaEyeLeft::Glaring => "ㆆ",
      MikaEyeLeft::Happy => "◜",
      MikaEyeLeft::Normal => "˙",
      MikaEyeLeft::PeerRight => "⚆",
      MikaEyeLeft::PeerStraight => "☉",
      MikaEyeLeft::Pleased => "◠",
      MikaEyeLeft::Resolved => "◡̀",
      MikaEyeLeft::RollingEyes => "◕",
      MikaEyeLeft::Sad => "◞",
      MikaEyeLeft::Scared => "Ͼ",
      MikaEyeLeft::Shades => "⌐▰",
      MikaEyeLeft::Sleeping => "⹇",
      MikaEyeLeft::Smiling => "ᗣ",
      MikaEyeLeft::Squinting => "≖",
      MikaEyeLeft::Surprised => "°",
      MikaEyeLeft::TearingUp => "ᗩ",
      MikaEyeLeft::Unimpressed => "¬",
      MikaEyeLeft::Wired => "◉",
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MikaEyeRight {
  Content,       // ˆ
  Confused,      // ಠ 
  Crying,        // ╥
  Dazed,         // ⋇
  Dead,          // ✖
  EyesSqueezed,  // ≺
  SuperSqueezed, // ᗕ
  Glaring,       // ㆆ
  Happy,         // ◝
  Normal,        // ˙
  PeerRight,     // ⚆
  PeerStraight,  // ☉
  Pleased,       // ◠
  Resolved,      // ◡́
  RollingEyes,   // ◕
  Sad,           // ◟
  Scared,        // Ͽ
  Shades,        // ▰
  Sleeping,      // ⹇
  Smiling,       // ᗣ
  Squinting,     // ≖
  Surprised,     // °
  TearingUp,     // ᗩ
  Unimpressed,   // ¬
  Wired,         // ◉
}

impl std::fmt::Display for MikaEyeRight {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.symbol())
  }
}

impl MikaEyeRight {
  pub fn symbol(&self) -> &'static str {
    match self {
      MikaEyeRight::Content => "ˆ",
      MikaEyeRight::Confused => "ಠ",
      MikaEyeRight::Crying => "╥",
      MikaEyeRight::Dazed => "⋇",
      MikaEyeRight::Dead => "✖",
      MikaEyeRight::EyesSqueezed => "≺",
      MikaEyeRight::SuperSqueezed => "ᗕ",
      MikaEyeRight::Glaring => "ㆆ",
      MikaEyeRight::Happy => "◝",
      MikaEyeRight::Normal => "˙",
      MikaEyeRight::PeerRight => "⚆",
      MikaEyeRight::PeerStraight => "☉",
      MikaEyeRight::Pleased => "◠",
      MikaEyeRight::Resolved => "◡́",
      MikaEyeRight::RollingEyes => "◕",
      MikaEyeRight::Sad => "◟",
      MikaEyeRight::Scared => "Ͽ",
      MikaEyeRight::Shades => "▰",
      MikaEyeRight::Sleeping => "⹇",
      MikaEyeRight::Smiling => "ᗣ",
      MikaEyeRight::Squinting => "≖",
      MikaEyeRight::Surprised => "°",
      MikaEyeRight::TearingUp => "ᗩ",
      MikaEyeRight::Unimpressed => "¬",
      MikaEyeRight::Wired => "◉",
    }
  }
}

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
  pub fn symbols(&self) -> (MikaEyeLeft, MikaNose, MikaEyeRight) {
    match self {
      MikaExpression::Content       => (MikaEyeLeft::Content,       MikaNose::Open,       MikaEyeRight::Content),
      MikaExpression::Confused      => (MikaEyeLeft::Confused,      MikaNose::Open,       MikaEyeRight::Confused),
      MikaExpression::Crying        => (MikaEyeLeft::Crying,        MikaNose::Open,       MikaEyeRight::Crying),
      MikaExpression::Dazed         => (MikaEyeLeft::Dazed,         MikaNose::Open,       MikaEyeRight::Dazed),
      MikaExpression::Dead          => (MikaEyeLeft::Dead,          MikaNose::Open,       MikaEyeRight::Dead),
      MikaExpression::EyesSqueezed  => (MikaEyeLeft::EyesSqueezed,  MikaNose::Open,       MikaEyeRight::EyesSqueezed),
      MikaExpression::Glaring       => (MikaEyeLeft::Glaring,       MikaNose::FlatMouth,  MikaEyeRight::Glaring),
      MikaExpression::Happy         => (MikaEyeLeft::Happy,         MikaNose::Open,       MikaEyeRight::Happy),
      MikaExpression::Normal        => (MikaEyeLeft::Normal,        MikaNose::Open,       MikaEyeRight::Normal),
      MikaExpression::PeerRight     => (MikaEyeLeft::PeerRight,     MikaNose::Open,       MikaEyeRight::PeerRight),
      MikaExpression::PeerStraight  => (MikaEyeLeft::PeerStraight,  MikaNose::Open,       MikaEyeRight::PeerStraight),
      MikaExpression::Pleased       => (MikaEyeLeft::Pleased,       MikaNose::Open,       MikaEyeRight::Pleased),
      MikaExpression::Resolved      => (MikaEyeLeft::Resolved,      MikaNose::Open,       MikaEyeRight::Resolved),
      MikaExpression::RollingEyes   => (MikaEyeLeft::RollingEyes,   MikaNose::Open,       MikaEyeRight::RollingEyes),
      MikaExpression::Sad           => (MikaEyeLeft::Sad,           MikaNose::Open,       MikaEyeRight::Sad),
      MikaExpression::Scared        => (MikaEyeLeft::Scared,        MikaNose::Open,       MikaEyeRight::Scared),
      MikaExpression::Shades        => (MikaEyeLeft::Shades,        MikaNose::Open,       MikaEyeRight::Shades),
      MikaExpression::Sleeping      => (MikaEyeLeft::Sleeping,      MikaNose::Open,       MikaEyeRight::Sleeping),
      MikaExpression::Smiling       => (MikaEyeLeft::Smiling,       MikaNose::Open,       MikaEyeRight::Smiling),
      MikaExpression::Squinting     => (MikaEyeLeft::Squinting,     MikaNose::Open,       MikaEyeRight::Squinting),
      MikaExpression::SuperSqueezed => (MikaEyeLeft::SuperSqueezed, MikaNose::Open,       MikaEyeRight::SuperSqueezed),
      MikaExpression::Surprised     => (MikaEyeLeft::Surprised,     MikaNose::Open,       MikaEyeRight::Surprised),
      MikaExpression::TearingUp     => (MikaEyeLeft::TearingUp,     MikaNose::Open,       MikaEyeRight::TearingUp),
      MikaExpression::Unimpressed   => (MikaEyeLeft::Unimpressed,   MikaNose::Open,       MikaEyeRight::Unimpressed),
      MikaExpression::Wired         => (MikaEyeLeft::Wired,         MikaNose::Open,       MikaEyeRight::Wired),
    }
  }
}

// MicroMika
// ---------------------------------------------------------------------------

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

  pub fn to_string(&self) -> String {
    let (left_arm, nose, right_arm) = self.symbols();
    format!("{}{}{}", left_arm.symbol(), nose.symbol(), right_arm.symbol())
  }

  pub fn symbols(&self) -> (MikaArm, MikaNose, MikaArm) {
    match self {
      MicroMika::Bat            => (MikaArm::BatWing,     MikaNose::Normal,  MikaArm::BatWing),
      MicroMika::BigHug         => (MikaArm::GestureLeft, MikaNose::Normal,  MikaArm::GestureRight),
      MicroMika::Cheer          => (MikaArm::RaisedLeft,  MikaNose::Normal,  MikaArm::RaisedRight),
      MicroMika::Dance          => (MikaArm::Dance,       MikaNose::Normal,  MikaArm::Dance),
      MicroMika::Goal           => (MikaArm::UpLeft,      MikaNose::Normal,  MikaArm::UpRight),
      MicroMika::GripperLeft    => (MikaArm::GripperLeft, MikaNose::Normal,  MikaArm::UpRight),
      MicroMika::GripperRight   => (MikaArm::UpLeft,      MikaNose::Normal,  MikaArm::GripperRight),
      MicroMika::GestureLeft    => (MikaArm::GestureLeft, MikaNose::Normal,  MikaArm::UpRight),
      MicroMika::GestureRight   => (MikaArm::UpLeft,      MikaNose::Normal,  MikaArm::GestureRight),
      MicroMika::Idle           => (MikaArm::Left,        MikaNose::Normal,  MikaArm::Right),
      MicroMika::Knight         => (MikaArm::Sword,       MikaNose::Normal,  MikaArm::Shield),
      MicroMika::Matrix         => (MikaArm::ShootLeft,   MikaNose::Normal,  MikaArm::ShootRight),
      MicroMika::OneWing        => (MikaArm::Sword,       MikaNose::Normal,  MikaArm::BatWing),
      MicroMika::PointLeft      => (MikaArm::Point,       MikaNose::Normal,  MikaArm::UpRight),
      MicroMika::PointRight     => (MikaArm::UpLeft,      MikaNose::Normal,  MikaArm::Point),
      MicroMika::Punch          => (MikaArm::PunchLeft,   MikaNose::Normal,  MikaArm::PunchLowRight),
      MicroMika::ShootLeft      => (MikaArm::ShootLeft,   MikaNose::Normal,  MikaArm::UpRight),
      MicroMika::ShootRight     => (MikaArm::UpLeft,      MikaNose::Normal,  MikaArm::ShootRight),
      MicroMika::Shrug          => (MikaArm::ShrugLeft,   MikaNose::Normal,  MikaArm::ShrugRight),
      MicroMika::ServeLeft      => (MikaArm::ShrugLeft,   MikaNose::Normal,  MikaArm::UpRight),
      MicroMika::ServeRight     => (MikaArm::UpLeft,      MikaNose::Normal,  MikaArm::ShrugRight),
      MicroMika::WaveLeft       => (MikaArm::UpLeft,      MikaNose::Normal,  MikaArm::UpRight),
      MicroMika::WaveRight      => (MikaArm::UpLeft,      MikaNose::Normal,  MikaArm::UpRight),
    }
  }
}


// Animations
// ---------------------------------------------------------------------------

// Sleep
static MIRCOMIKA_POWEROFF: &[&str] = &["╭⦿╮","╭⦾╮","╭⊚╮","╭⊙╮","╭◯╮"];
static MIRCOMIKA_POWERON: &[&str] = &["╭◯╮","╭⊙╮","╭⊚╮","╭⦾╮","╭⦿╮"];
static MICROMIKA_BLINK: &[&str] = &["╭⦿╮","╭⊖╮","╭⦿╮"];
static MICROMIKA_PULSE: &[&str] = &["╭⦿╮","╭⦾╮","╭⊚╮","╭⊙╮","╭⊚╮","╭⦾╮","╭⦿╮"];
static MICROMIKA_WAVE: &[&str] = &["╭⦿╯","╭⦿─",];
static MICROMIKA_RAISE_ARMS: &[&str] = &["╭⦿╮","─⦿─","╰⦿╯"];
static MICROMIKA_LOWER_ARMS: &[&str] = &["╰⦿╯","─⦿─","╭⦿╮"];
static MICROMIKA_FLAPPING: &[&str] = &["─⦿─","╰⦿╯"];
static MICROMIKA_GRIPPING_RIGHT: &[&str] = &["╭⦿─‹ -> ╭⦿─-"];
static MICROMIKA_GRIPPING_LEFT: &[&str] = &["›─⦿╮ -> -─⦿╮"];

// Mylo
// ---------------------------------------------------------------------------

// Mylo is a secondary character, he's under development right now on the basis of these faces. Maybe he's a villain? Maybe he's Mika's siblng? I don't know.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MyloExpression {
  Eyes,       // (ᑕ⎔ᑐ)
  Focused,    // (ᕮ⎔ᕭ)
  Alarm,      // (ᕳ⎔ᕲ)
  Angry,      // (ᘭ⎔ᘪ)
  Crossed,    // (ᑢ⎔ᑝ)
}

impl MyloExpression {
  pub fn symbols(&self) -> (&'static str, &'static str, &'static str) {
    match self {
      MyloExpression::Eyes => ("ᑕ", "⎔", "ᑐ"),
      MyloExpression::Focused => ("ᕮ", "⎔", "ᕭ"),
      MyloExpression::Alarm => ("ᕳ", "⎔", "ᕲ"),
      MyloExpression::Angry => ("ᘭ", "⎔", "ᘪ"),
      MyloExpression::Crossed => ("ᑢ", "⎔", "ᑝ"),
    }
  }
}