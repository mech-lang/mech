use crate::*;

// Mika
// ============================================================================

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MikaExpression {
  Content,       // (ˆ◯ˆ)
  Confused,      // (ಠ◯ಠ) 
  Crying,        // (╥◯╥)
  Dazed,         // (⋇◯⋇)
  Dead,          // (✖◯✖)
  EyesSqueezed,  // (≻◯≺)
  Glaring,       // (ㆆ⍜ㆆ)
  Happy,         // (◜◯◝)
  Normal,        // (˙◯˙)
  PeerRight,     // (⚆◯⚆)
  PeerStraight,  // (☉◯☉)
  Pleased        // (◠◯◠)
  Resolved,      // (◡̀◯◡́)ᕤ
  RollingEyes,   // (◕◯◕)
  Sad,           // (◞◯◟)
  Scared,        // (Ͼ◯Ͽ)
  Shades,        // (⌐▰◯▰)
  Sleeping,      // (-◯-) 「Hello Welcome to Mech! I'm Mika!」
  Squinting,     // (≖◯≖)
  Surprised,     // (°◯°)
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

pub enum SmallMika {
  Idle,        // ╭⦿╮
  Pointing,    // ╭⦿─
  Waving,      // ╭⦿╯
  Cheering,    // ╰⦿╯
  Shrug,       // -◡⦿◡-
  Behind,      // ╭⊕╮
  Off,         // ╭◯╮
  Gripper,     // ╭⦿─‹
  BigHug,      // ›⌣⦿⌣‹
  Knight,      // ⸸⦿ᗢ
  Matrix,      // ·¬⦿⌐·
  Dancing,     // ~⦿~
}