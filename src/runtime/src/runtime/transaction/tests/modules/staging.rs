use super::super::RuntimeModuleJournal;
use super::module;
use crate::{
    ModuleRecord, ModuleVersionId, ModuleVersionRecord, RuntimeModuleJournalConflict, module_id,
};

#[test]
fn module_staging_ignores_description_differences() {
    let mut journal = RuntimeModuleJournal::new();
    let first = module("memory://module.mec", "first");
    let second = module("memory://module.mec", "second");

    assert!(journal.stage_module(first.clone()).unwrap());
    assert!(!journal.stage_module(second).unwrap());
    assert_eq!(
        journal.get_module(first.id).unwrap().description.as_deref(),
        Some("first"),
    );
    assert_eq!(journal.mark(), 1);
}

#[test]
fn conflicting_module_identity_fails_without_mutation() {
    let mut journal = RuntimeModuleJournal::new();
    let first = module("memory://module.mec", "first");
    journal.stage_module(first.clone()).unwrap();
    let mark = journal.mark();
    let conflict = ModuleRecord::new(first.id, "memory://other.mec");

    let error = journal.stage_module(conflict).unwrap_err();

    assert!(error.kind_as::<RuntimeModuleJournalConflict>().is_some(),);
    assert_eq!(journal.mark(), mark);
    assert_eq!(journal.get_module(first.id), Some(&first));
}

#[test]
fn conflicting_version_identity_fails_without_mutation() {
    let mut journal = RuntimeModuleJournal::new();
    let owner = module_id("memory://module.mec");
    let first = ModuleVersionRecord::new(ModuleVersionId(10), owner, 1);
    journal.stage_version(first.clone()).unwrap();
    let mark = journal.mark();
    let conflict = ModuleVersionRecord::new(ModuleVersionId(10), owner, 2);

    let error = journal.stage_version(conflict).unwrap_err();

    assert!(error.kind_as::<RuntimeModuleJournalConflict>().is_some(),);
    assert_eq!(journal.mark(), mark);
    assert_eq!(journal.get_version(first.id), Some(&first));
}
