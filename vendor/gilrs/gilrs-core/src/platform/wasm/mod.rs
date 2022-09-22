mod ff;
mod gamepad;

pub use self::ff::Device as FfDevice;
pub use self::gamepad::{native_ev_codes, EvCode, Gamepad, Gilrs};

pub const IS_Y_AXIS_REVERSED: bool = true;
