#![allow(warnings)]

#[cfg(feature = "browser_project_runner")]
mod project;

#[cfg(feature = "browser_project_runner")]
pub use project::*;
