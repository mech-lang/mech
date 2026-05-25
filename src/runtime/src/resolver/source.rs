use mech_core::MechErrorKind;

use std::path::Path;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SourceKind {
  Mech,
  MechBytecode,
  MechDocument,
  MechPackage,
  Html,
  Css,
  Markdown,
  Csv,
  JavaScript,
  Image(String),
  Matlab,
  Unknown(String),
}

impl SourceKind {
  pub fn from_path(path: &Path) -> Self {
    let extension = path
      .extension()
      .and_then(|extension| extension.to_str())
      .unwrap_or("")
      .to_ascii_lowercase();

    Self::from_extension(&extension)
  }

  pub fn from_extension(extension: &str) -> Self {
    match extension {
      "mec" | "🤖" => Self::Mech,
      "mecb" => Self::MechBytecode,
      "mdoc" => Self::MechDocument,
      "mpkg" => Self::MechPackage,
      "html" | "htm" => Self::Html,
      "css" => Self::Css,
      "md" => Self::Markdown,
      "csv" => Self::Csv,
      "js" => Self::JavaScript,
      "m" => Self::Matlab,
      "png" | "jpg" | "jpeg" | "gif" | "svg" => {
        Self::Image(extension.to_string())
      }
      "" => Self::Unknown("".to_string()),
      other => Self::Unknown(other.to_string()),
    }
  }

  pub fn is_executable_mech(&self) -> bool {
    matches!(self, Self::Mech | Self::MechBytecode)
  }

  pub fn is_text_asset(&self) -> bool {
    matches!(
      self,
      Self::Mech
        | Self::MechDocument
        | Self::MechPackage
        | Self::Html
        | Self::Css
        | Self::Markdown
        | Self::Csv
        | Self::JavaScript
        | Self::Matlab
    )
  }
}

#[derive(Debug, Clone)]
pub struct SourceFileOpenFailed {
  pub path: String,
  pub source: String,
}

impl MechErrorKind for SourceFileOpenFailed {
  fn name(&self) -> &str {
    "SourceFileOpenFailed"
  }

  fn message(&self) -> String {
    format!(
      "Could not open source file `{}`: {}",
      self.path,
      self.source,
    )
  }
}

#[derive(Debug, Clone)]
pub struct SourceFileReadFailed {
  pub path: String,
  pub source: String,
}

impl MechErrorKind for SourceFileReadFailed {
  fn name(&self) -> &str {
    "SourceFileReadFailed"
  }

  fn message(&self) -> String {
    format!(
      "Could not read source file `{}`: {}",
      self.path,
      self.source,
    )
  }
}

#[derive(Debug, Clone)]
pub struct SourceExtensionDecodeFailed {
  pub path: String,
}

impl MechErrorKind for SourceExtensionDecodeFailed {
  fn name(&self) -> &str {
    "SourceExtensionDecodeFailed"
  }

  fn message(&self) -> String {
    format!("Could not decode source file extension for `{}`", self.path)
  }
}

#[derive(Debug, Clone)]
pub struct SourceUnknownFileExtension {
  pub path: String,
  pub extension: String,
}

impl MechErrorKind for SourceUnknownFileExtension {
  fn name(&self) -> &str {
    "SourceUnknownFileExtension"
  }

  fn message(&self) -> String {
    format!(
      "Unknown source file extension `{}` for `{}`",
      self.extension,
      self.path,
    )
  }
}

#[derive(Debug, Clone)]
pub struct SourceIncludeReadFailed {
  pub path: String,
  pub include: String,
  pub source: String,
}

impl MechErrorKind for SourceIncludeReadFailed {
  fn name(&self) -> &str {
    "SourceIncludeReadFailed"
  }

  fn message(&self) -> String {
    format!(
      "Could not read include `{}` from `{}`: {}",
      self.include,
      self.path,
      self.source,
    )
  }
}

#[derive(Debug, Clone)]
pub struct SourceIncludeCycle {
  pub path: String,
}

impl MechErrorKind for SourceIncludeCycle {
  fn name(&self) -> &str {
    "SourceIncludeCycle"
  }

  fn message(&self) -> String {
    format!("Source include cycle detected at `{}`", self.path)
  }
}