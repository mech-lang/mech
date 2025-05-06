#![no_main]
#![allow(warnings)]
#[macro_use]
extern crate mech_core;
extern crate nalgebra as na;
extern crate paste;

use paste::paste;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};

pub mod print;
pub mod println;

pub use self::print::*;
pub use self::println::*;