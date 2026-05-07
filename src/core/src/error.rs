use crate::*;
use std::sync::Arc;
use std::any::Any;

// Errors
// ----------------------------------------------------------------------------

type Rows = usize;
type Cols = usize;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilerSourceRange {
  pub file: &'static str,
  pub line: u32,
}

impl CompilerSourceRange {
  #[track_caller]
  pub fn here() -> Self {
    let loc = std::panic::Location::caller();
    Self {
      file: loc.file(),
      line: loc.line(),
    }
  }
}

#[macro_export]
macro_rules! compiler_loc {
  () => {
    $crate::CompilerSourceRange {
      file: file!(),
      line: line!(),
    }
  };
}

trait ErrorKindCallbacks: Send + Sync {
  fn name(&self, data: &dyn Any) -> String;
  fn message(&self, data: &dyn Any) -> String;
}

struct CallbacksImpl<K> {
  // zero-sized; all behavior encoded in trait impl below
  _marker: std::marker::PhantomData<K>,
}

impl<K> CallbacksImpl<K> {
  fn new() -> Self {
    Self { _marker: std::marker::PhantomData }
  }
}

impl<K> ErrorKindCallbacks for CallbacksImpl<K>
where
  K: MechErrorKind + 'static,
{
  fn name(&self, data: &dyn Any) -> String {
    // downcast and call name()
    let k = data.downcast_ref::<K>().expect("wrong kind type in vtable");
    k.name().to_string()
  }

  fn message(&self, data: &dyn Any) -> String {
    let k = data.downcast_ref::<K>().expect("wrong kind type in vtable");
    k.message()
  }
}

pub trait MechErrorKind: std::fmt::Debug + Send + Sync + Clone {
  fn name(&self) -> &str;
  fn message(&self) -> String;
}

//#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
//#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[derive(Clone)]
pub struct MechError {
  kind_data: Arc<dyn Any + Send + Sync>,
  kind_callbacks: Arc<dyn ErrorKindCallbacks>, // object-safe vtable
  pub program_range: Option<SourceRange>,
  pub annotations: Vec<SourceRange>,
  pub tokens: Vec<Token>,
  pub compiler_location: Option<CompilerSourceRange>,
  pub source: Option<Box<MechError>>, // for propagation
  pub message: Option<String>,
}

impl std::fmt::Debug for MechError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("MechError")
      .field("kind name", &self.kind_name())
      .field("kind message", &self.kind_message())
      .field("message", &self.message)
      .field("program_range", &self.program_range)
      .field("annotations", &self.annotations)
      .field("tokens", &self.tokens)
      .field("compiler_location", &self.compiler_location)
      .field("source", &self.source)
      .finish()
  }
}

impl MechError {
  pub fn new<K: MechErrorKind + 'static>(
    kind: K,
    message: Option<String>
  ) -> Self {
    MechError {
      kind_data: Arc::new(kind),
      kind_callbacks: Arc::new(CallbacksImpl::<K>::new()),
      program_range: None,
      annotations: Vec::new(),
      tokens: Vec::new(),
      compiler_location: None,
      source: None,
      message,
    }
  }

  /// Get the kind as a specific type, if it matches
  pub fn kind_as<K: MechErrorKind + 'static>(&self) -> Option<&K> {
    self.kind_data.downcast_ref::<K>()
  }

  /// Get the runtime name (delegates to the underlying kind)
  pub fn kind_name(&self) -> String {
    self.kind_callbacks.name(self.kind_data.as_ref())
  }

  /// Get the runtime message (delegates to the underlying kind)
  pub fn kind_message(&self) -> String {
    self.kind_callbacks.message(self.kind_data.as_ref())
  }

  /// Optional helper that returns the message or the explicit `message` override
  pub fn display_message(&self) -> String {
    if let Some(ref m) = self.message { m.clone() } else { self.kind_message() }
  }

  /// If you ever need downcast access to the concrete kind:
  pub fn kind_downcast_ref<K: 'static>(&self) -> Option<&K> {
    self.kind_data.downcast_ref::<K>()
  }

  /// Add a compiler location annotation with the current location
  #[track_caller]
  pub fn with_compiler_loc(mut self) -> Self {
    self.compiler_location = Some(CompilerSourceRange::here());
    self
  }

  /// Specify a particular compiler location
  pub fn with_specific_compiler_loc(mut self, loc: CompilerSourceRange) -> Self {
    self.compiler_location = Some(loc);
    self
  }

  /// Add a source range annotation
  pub fn with_annotation(mut self, range: SourceRange) -> Self {
    self.annotations.push(range);
    self
  }

  /// Add multiple source range annotations
  pub fn with_annotations<I>(mut self, iter: I) -> Self
  where
    I: IntoIterator<Item = SourceRange>,
  {
    self.annotations.extend(iter);
    self
  }

  /// Add tokens related to this error
  pub fn with_tokens<I>(mut self, iter: I) -> Self
  where
    I: IntoIterator<Item = Token>,
  {
    self.tokens.extend(iter);
    self
  }

  /// Set the source error that caused this one
  pub fn with_source(mut self, src: MechError) -> Self {
    self.source = Some(Box::new(src));
    self
  }

  /// Get the primary source range associated with this error
  pub fn primary_range(&self) -> Option<SourceRange> {
    self.program_range.clone()
  }

  /// Get a simple message describing the error
  pub fn simple_message(&self) -> String {
    format!("{}: {}", self.kind_name(), self.kind_message())
  }

  /// Get a full chain message including all source errors
  pub fn full_chain_message(&self) -> String {
    let mut out = self.simple_message();
    let mut current = &self.source;

    while let Some(err) = current {
      out.push_str("\nCaused by: ");
      out.push_str(&err.simple_message());
      current = &err.source;
    }

    out
  }

  pub fn boxed(self) -> Box<Self> {
    Box::new(self)
  }

  #[cfg(feature = "pretty_print")]
  pub fn to_html(&self) -> String {
    fn escape_html(input: &str) -> String {
      input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
    }

    let source_range = self
      .program_range
      .clone()
      .or_else(|| self.tokens.first().map(|token| token.src_range.clone()));

    let token_html = if self.tokens.is_empty() {
      String::new()
    } else {
      let token_list = self
        .tokens
        .iter()
        .map(|token| format!(
          "<li><span class=\"mech-runtime-error-token\">{}</span> <span class=\"mech-runtime-error-token-range\">[{}:{}, {}:{})</span></li>",
          escape_html(&token.to_string()),
          token.src_range.start.row,
          token.src_range.start.col,
          token.src_range.end.row,
          token.src_range.end.col
        ))
        .collect::<Vec<String>>()
        .join("");
      format!(
        "<div class=\"mech-runtime-error-section\"><div class=\"mech-runtime-error-section-title\">Source tokens</div><ul class=\"mech-runtime-error-list\">{}</ul></div>",
        token_list
      )
    };

    let mut causes = vec![];
    let mut current = &self.source;
    while let Some(err) = current {
      causes.push(format!(
        "<li>{}</li>",
        escape_html(&format!("{}: {}", err.kind_name(), err.display_message()))
      ));
      current = &err.source;
    }
    let causes_html = if causes.is_empty() {
      String::new()
    } else {
      format!(
        "<div class=\"mech-runtime-error-section\"><div class=\"mech-runtime-error-section-title\">Caused by</div><ul class=\"mech-runtime-error-list\">{}</ul></div>",
        causes.join("")
      )
    };

    let compiler_location_html = match &self.compiler_location {
      Some(loc) => format!(
        "<div class=\"mech-runtime-error-meta-row\"><span class=\"mech-runtime-error-meta-label\">Compiler location</span><span class=\"mech-runtime-error-meta-value\">{}:{}</span></div>",
        escape_html(loc.file),
        loc.line
      ),
      None => String::new(),
    };

    format!(
      "<div class=\"mech-runtime-error\">
        <div class=\"mech-runtime-error-header\">
          <span class=\"mech-runtime-error-icon\" aria-hidden=\"true\"></span>
          <div>
            <div class=\"mech-runtime-error-title\">{}</div>
            <div class=\"mech-runtime-error-message\">{}</div>
          </div>
        </div>
      <div class=\"mech-runtime-error-meta\">{}{}</div>
      {}
      </div>",
      escape_html(&self.kind_name()),
      escape_html(&self.display_message()),
      token_html,
      compiler_location_html,
      causes_html
    )
  }
}

#[derive(Debug, Clone)]
pub struct UndefinedKindError {
  pub kind_id: u64,
}
impl MechErrorKind for UndefinedKindError {
  fn name(&self) -> &str {
    "UndefinedKind"
  }
  fn message(&self) -> String {
    format!("Kind `{}` is not defined.", self.kind_id)
  }
}

impl From<std::io::Error> for MechError {
  fn from(err: std::io::Error) -> Self {
    MechError::new(
      IoErrorWrapper { msg: err.to_string() },
      None
    )
    .with_compiler_loc()
  }
}

#[derive(Debug, Clone)]
pub struct DimensionMismatch {
  pub dims: Vec<usize>,
}
impl MechErrorKind for DimensionMismatch {
  fn name(&self) -> &str { "DimensionMismatch" }
  fn message(&self) -> String { format!("Matrix dimension mismatch: {:?}", self.dims) }
}

#[derive(Debug, Clone)]
pub struct GenericError {
  pub msg: String,
}
impl MechErrorKind for GenericError {
  fn name(&self) -> &str { "GenericError" }

  fn message(&self) -> String {
    format!("Error: {}", self.msg)
  }
}

#[derive(Debug, Clone)]
pub struct FeatureNotEnabledError;
impl MechErrorKind for FeatureNotEnabledError {
  fn name(&self) -> &str { "FeatureNotEnabled" }

  fn message(&self) -> String {
    format!("Feature not enabled")
  }
}

#[derive(Debug, Clone)]
pub struct NotExecutableError {}
impl MechErrorKind for NotExecutableError {
  fn name(&self) -> &str { "NotExecutable" }

  fn message(&self) -> String {
    format!("Not executable")
  }
}

#[derive(Debug, Clone)]
pub struct IoErrorWrapper {
  pub msg: String,
}
impl MechErrorKind for IoErrorWrapper {
  fn name(&self) -> &str { "IoError" }

  fn message(&self) -> String {
    format!("IO error: {}", self.msg)
  }
}