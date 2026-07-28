mod rollback;
mod staging;

use super::{ModuleRecord, module_id};

pub(super) fn module(uri: &str, description: &str) -> ModuleRecord {
    ModuleRecord::new(module_id(uri), uri).with_description(description)
}
