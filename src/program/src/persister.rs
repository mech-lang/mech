use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Persister {
  pub path: PathBuf,
}

impl Persister {
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self { path: path.into() }
  }
}
