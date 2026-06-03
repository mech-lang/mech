mod error;
mod extract;
mod ir;
mod compile;
mod analyze;
mod eval;
mod lower;

pub use error::*;
pub use extract::*;
pub use ir::*;
pub use compile::*;
pub use analyze::*;
pub use eval::*;
pub use lower::*;

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
    let ir = ConfigCompiler::new(options.clone()).compile(&extracted)?;
    ConfigAnalyzer::new(options.clone()).analyze(&ir)?;
    let value = ConfigEvaluator::new(options).evaluate(&ir)?;
    ConfigLowerer::new().lower(source_name.into(), value)
}
