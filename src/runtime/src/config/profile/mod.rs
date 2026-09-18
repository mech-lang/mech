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
use mech_core::{GenericError, MResult, MechError};

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
    use std::sync::Arc;

    use mech_syntax::document::{ParseConfig, Revision};

    let source_name = source_name.into();
    let document = crate::resolver::SourceDocument::parse_resolved(
        &source_name,
        Revision(0),
        Arc::<str>::from(source),
        ParseConfig::default(),
    )
    .map_err(|error| {
        MechError::new(
            GenericError {
                msg: format!("unable to retain configuration source: {error}"),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    compile_config_document(source_name, &document, options)
}

/// Compile retained canonical configuration through the existing restricted IR,
/// analyzer, evaluator and field lowering. No source parse or general evaluator
/// is introduced at this boundary. The shipping text route switches in S8C.
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
