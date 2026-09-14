#[cfg(feature = "source")]
mod analyze;
#[cfg(feature = "source")]
mod canonical;
mod error;
mod eval;
#[cfg(feature = "source")]
mod ir;
mod lower;

#[cfg(feature = "source")]
use self::analyze::ConfigAnalyzer;
pub use self::error::InvalidConfigField;
#[cfg(feature = "source")]
pub use self::error::InvalidConfigSyntax;
#[cfg(feature = "source")]
use self::error::*;
#[cfg(feature = "source")]
use self::eval::ConfigEvaluator;
pub use self::eval::ConfigValue;
#[cfg(feature = "source")]
use self::ir::{ConfigExpr, ConfigFunction, ConfigItem, ConfigLet, ConfigProgram};
#[cfg(feature = "source")]
use self::lower::ConfigLowerer;
pub use self::lower::{
    ActorBootstrapConfig, BuildHostConfig, ConfigCapabilityGrant, ConfigCapabilityKind,
    DiagnosticsConfigPatch, MechConfigDocument, RunHostConfig, RuntimeConfigPatch,
    RuntimeLimitsPatch, ServeHostConfig, ServePresentation,
};

#[cfg(feature = "source")]
use mech_core::MResult;

pub const DEFAULT_CONFIG_FILENAME: &str = "mech.mcfg";

#[derive(Clone, Debug)]
pub struct ConfigProfileOptions {
    pub executable_namespaces: Vec<String>,
    pub max_eval_steps: usize,
    pub max_function_depth: usize,
    pub max_collection_items: usize,
    pub max_string_bytes: usize,
}

impl Default for ConfigProfileOptions {
    fn default() -> Self {
        Self {
            executable_namespaces: vec!["config".to_string(), "mech-config".to_string()],
            max_eval_steps: 10_000,
            max_function_depth: 32,
            max_collection_items: 10_000,
            max_string_bytes: 1024 * 1024,
        }
    }
}

#[cfg(feature = "source")]
pub fn parse_config_document(
    source_name: impl Into<String>,
    source: &str,
    options: ConfigProfileOptions,
) -> MResult<MechConfigDocument> {
    use mech_syntax::document::{DocumentId, ParseConfig, Revision, TextSnapshot};

    // This one-shot configuration has no cache identity. Callers retaining an
    // editor or stream revision use compile_config_document directly.
    let text = TextSnapshot::new(DocumentId(0), Revision(0), source).map_err(|error| {
        ConfigProfileViolation::error(format!("invalid configuration source: {error:?}"))
    })?;
    let document = crate::resolver::SourceDocument::parse(text, ParseConfig::default());
    compile_config_document(source_name, &document, options)
}

/// Compile retained canonical configuration through the existing restricted IR,
/// analyzer, evaluator and field lowering. No source parse or general evaluator
/// is introduced at this boundary. Text configuration and retained revisions
/// share this compiler.
#[cfg(feature = "source")]
pub fn compile_config_document(
    source_name: impl Into<String>,
    source: &crate::resolver::SourceDocument,
    options: ConfigProfileOptions,
) -> MResult<MechConfigDocument> {
    let source_name = source_name.into();
    if !source.is_strictly_clean() {
        return Err(mech_core::MechError::new(
            InvalidConfigSyntax {
                source_name,
                source: source.clone(),
            },
            None,
        ));
    }
    let ir = canonical::compile(&source.document(), &options)?;
    ConfigAnalyzer::new().analyze(&ir)?;
    let value = ConfigEvaluator::new(options).evaluate(&ir)?;
    ConfigLowerer::new().lower(source_name, value)
}
