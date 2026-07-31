mod analyze;
mod compile;
mod error;
mod eval;
mod extract;
mod ir;
mod lower;

use self::analyze::ConfigAnalyzer;
use self::compile::ConfigCompiler;
use self::error::*;
pub use self::error::InvalidConfigField;
use self::eval::ConfigEvaluator;
pub use self::eval::ConfigValue;
use self::extract::{ConfigExtractor, ExtractedConfigProgram};
use self::ir::{
    ConfigExpr, ConfigFunction, ConfigItem, ConfigLet, ConfigProgram,
};
use self::lower::ConfigLowerer;
pub use self::lower::{
    ConfigCapabilityGrant, ConfigCapabilityKind, DiagnosticsConfigPatch, MechConfigDocument,
    RunHostConfig, RuntimeConfigPatch, RuntimeLimitsPatch, ServeHostConfig,
};

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
