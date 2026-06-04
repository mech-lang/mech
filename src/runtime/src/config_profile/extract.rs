use mech_core::{Comment, MResult, MechCode, Program, SectionElement};

use super::{ConfigProfileOptions, ConfigProfileViolation};

#[derive(Clone, Debug)]
pub struct ExtractedConfigProgram {
    pub code: Vec<(MechCode, Option<Comment>)>,
}

pub struct ConfigExtractor {
    options: ConfigProfileOptions,
}

impl ConfigExtractor {
    pub fn new(options: ConfigProfileOptions) -> Self {
        Self { options }
    }

    pub fn extract(&self, program: &Program) -> MResult<ExtractedConfigProgram> {
        let mut code = Vec::new();
        for section in &program.body.sections {
            for element in &section.elements {
                match element {
                    SectionElement::MechCode(items) => code.extend(items.iter().cloned()),
                    SectionElement::FencedMechCode(fenced)
                        if self
                            .options
                            .executable_namespaces
                            .iter()
                            .any(|ns| ns == &fenced.config.namespace_str) =>
                    {
                        if !fenced.imports.is_empty() {
                            return Err(ConfigProfileViolation::error(
                                "imports are not allowed in Mech config",
                            ));
                        }
                        if !fenced.exports.is_empty() {
                            return Err(ConfigProfileViolation::error(
                                "exports are not allowed in Mech config",
                            ));
                        }
                        code.extend(fenced.code.iter().cloned());
                    }
                    _ => {}
                }
            }
        }
        Ok(ExtractedConfigProgram { code })
    }
}
