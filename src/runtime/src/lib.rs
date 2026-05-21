#![cfg_attr(feature = "no_std", no_std)]

#[cfg(feature = "serde")]
use serde_derive::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeInfo {
  pub name: &'static str,
}

impl RuntimeInfo {
  pub const fn new(name: &'static str) -> Self {
    Self { name }
  }
}
