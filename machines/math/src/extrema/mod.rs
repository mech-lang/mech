#[macro_use]
pub use crate::*;
#[cfg(feature = "fmax")]
pub mod fmax;
#[cfg(feature = "fmaximum_num")]
pub mod fmaximum_num;
#[cfg(feature = "fmaximum")]
pub mod fmaximum;
#[cfg(feature = "fmin")]
pub mod fmin;
#[cfg(feature = "fminimum_num")]
pub mod fminimum_num;
#[cfg(feature = "fminimum")]
pub mod fminimum;

#[cfg(feature = "fmax")]
pub use self::fmax::*;
#[cfg(feature = "fmaximum_num")]
pub use self::fmaximum_num::*;
#[cfg(feature = "fmaximum")]
pub use self::fmaximum::*;
#[cfg(feature = "fmin")]
pub use self::fmin::*;
#[cfg(feature = "fminimum_num")]
pub use self::fminimum_num::*;
#[cfg(feature = "fminimum")]
pub use self::fminimum::*;