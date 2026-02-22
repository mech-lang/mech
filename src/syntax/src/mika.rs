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

