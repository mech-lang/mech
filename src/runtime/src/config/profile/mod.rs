#[cfg(feature = "source")]
mod analyze;
#[cfg(feature = "source")]
mod compile;
mod error;
mod eval;
#[cfg(feature = "source")]
mod extract;
#[cfg(feature = "source")]
mod ir;
mod lower;

#[cfg(feature = "source")]
use self::analyze::ConfigAnalyzer;
#[cfg(feature = "source")]
use self::compile::ConfigCompiler;
pub use self::error::InvalidConfigField;
#[cfg(feature = "source")]
use self::error::*;
#[cfg(feature = "source")]
use self::eval::ConfigEvaluator;
pub use self::eval::ConfigValue;
#[cfg(feature = "source")]
use self::extract::{ConfigExtractor, ExtractedConfigProgram};
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
    let program = mech_syntax::parser::parse(source)?;
    let extracted = ConfigExtractor::new(options.clone()).extract(&program)?;
    let ir = ConfigCompiler::new().compile(&extracted)?;
    ConfigAnalyzer::new().analyze(&ir)?;
    let value = ConfigEvaluator::new(options).evaluate(&ir)?;
    ConfigLowerer::new().lower(source_name.into(), value)
}
