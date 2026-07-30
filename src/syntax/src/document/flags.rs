use core::ops::{BitOr, BitOrAssign};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

macro_rules! flags {
  ($name:ident, $storage:ty) => {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
    #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
    pub struct $name(pub $storage);

    impl $name {
      pub const NONE: Self = Self(0);

      pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
      }

      pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
      }
    }

    impl BitOr for $name {
      type Output = Self;

      fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
      }
    }

    impl BitOrAssign for $name {
      fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
      }
    }
  };
}

flags!(NodeFlags, u16);
flags!(TokenFlags, u16);

impl NodeFlags {
  pub const ERROR: Self = Self(1 << 0);
  pub const MISSING: Self = Self(1 << 1);
  pub const REPARSE_ROOT: Self = Self(1 << 2);
  pub const CONTAINS_ERROR: Self = Self(1 << 3);
  pub const CONTAINS_MISSING: Self = Self(1 << 4);
}

impl TokenFlags {
  pub const SYNTHETIC: Self = Self(1 << 0);
  pub const MISSING: Self = Self(1 << 1);
  pub const ERROR: Self = Self(1 << 2);
  pub const TRIVIA: Self = Self(1 << 3);
}

#[cfg(test)]
mod tests {
  use super::TokenFlags;

  #[test]
  fn trivia_is_an_independent_composable_token_flag() {
    assert_eq!(TokenFlags::TRIVIA.0, 1 << 3);
    assert!(!TokenFlags::TRIVIA.intersects(TokenFlags::SYNTHETIC));
    assert!(!TokenFlags::TRIVIA.intersects(TokenFlags::MISSING));
    assert!(!TokenFlags::TRIVIA.intersects(TokenFlags::ERROR));

    let synthetic_trivia = TokenFlags::SYNTHETIC | TokenFlags::TRIVIA;
    assert!(synthetic_trivia.contains(TokenFlags::SYNTHETIC));
    assert!(synthetic_trivia.contains(TokenFlags::TRIVIA));
  }
}
