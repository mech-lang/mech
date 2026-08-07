use std::collections::{HashMap, HashSet, VecDeque};

use mech_core::{MResult, MechError};

use super::{
    InMemoryStore, InvalidStoreRecordError, ModuleId, ModuleRecord, ModuleVersionId,
    ModuleVersionRecord, RuntimeStoreCommit, StoreCapabilityNotRevocableError,
    StoreRecordAlreadyExistsError, StoreRecordNotFoundError, module_id,
};

pub(super) struct RuntimeCommitValidation {
    pub(super) module_inserts: Vec<bool>,
    pub(super) module_version_inserts: Vec<bool>,
    pub(super) capability_subjects: Vec<String>,
}

pub(super) fn validate_runtime_commit(
    store: &InMemoryStore,
    commit: &RuntimeStoreCommit,
) -> MResult<RuntimeCommitValidation> {
    let module_inserts = validate_modules(store, &commit.module_puts)?;
    let module_version_inserts = validate_module_versions(
        store,
        &commit.module_puts,
        &commit.module_version_puts,
        &module_inserts,
    )?;

    let mut object_puts = HashSet::with_capacity(commit.object_puts.len());
    for object in &commit.object_puts {
        object.validate()?;
        if store.objects.contains_key(&object.id) || !object_puts.insert(object.id) {
            return already_exists("object", object.id.to_string());
        }
    }
    for object in &commit.object_updates {
        object.validate()?;
        if !store.objects.contains_key(&object.id) && !object_puts.contains(&object.id) {
            return not_found("object", object.id.to_string());
        }
    }

    for task in &commit.task_updates {
        task.validate()?;
        if !store.tasks.contains_key(&task.id) {
            return not_found("task", task.id.to_string());
        }
    }

    for actor in &commit.actor_updates {
        actor.validate()?;
        if !store.actors.contains_key(&actor.id) {
            return not_found("actor", actor.id.to_string());
        }
    }

    for (actor, _) in &commit.message_acks {
        if !store.actors.contains_key(actor) {
            return not_found("actor", actor.to_string());
        }
    }
    for (actor, message) in &commit.message_enqueues {
        if !store.actors.contains_key(actor) {
            return not_found("actor", actor.to_string());
        }
        message.validate()?;
        if message.actor != *actor {
            return Err(MechError::new(
                InvalidStoreRecordError {
                    field: "message.actor",
                    reason: "message actor does not match target actor",
                },
                None,
            ));
        }
    }

    let mut staged_capabilities = HashMap::with_capacity(commit.capability_grants.len());
    let mut capability_subjects = Vec::with_capacity(commit.capability_grants.len());
    for (id, capability) in &commit.capability_grants {
        capability.validate()?;
        if id.is_zero() {
            return super::invalid_store_record("capability.id", "must not be zero");
        }
        if capability.id() != *id {
            return Err(MechError::new(
                InvalidStoreRecordError {
                    field: "capability.id",
                    reason: "capability id does not match grant id",
                },
                None,
            ));
        }
        if store.capabilities.contains_key(id) || staged_capabilities.contains_key(id) {
            return already_exists("capability", id.to_string());
        }
        capability_subjects.push(capability.subject_key().to_string());
        staged_capabilities.insert(*id, capability);
    }
    for id in &commit.capability_revocations {
        let capability = staged_capabilities
            .get(id)
            .copied()
            .or_else(|| store.capabilities.get(id));
        let Some(capability) = capability else {
            return not_found("capability", id.to_string());
        };
        if !capability.is_revocable() {
            return Err(MechError::new(
                StoreCapabilityNotRevocableError { capability: *id },
                None,
            ));
        }
    }

    validate_events(store, &commit.events)?;

    commit.transaction.validate()?;
    if store.transactions.contains_key(&commit.transaction.id) {
        return already_exists("transaction", commit.transaction.id.to_string());
    }

    Ok(RuntimeCommitValidation {
        module_inserts,
        module_version_inserts,
        capability_subjects,
    })
}

fn validate_events(store: &InMemoryStore, events: &[super::RuntimeEvent]) -> MResult<()> {
    let mut pruned_store_ids = HashSet::with_capacity(events.len());
    let mut staged_ids = HashSet::with_capacity(events.len());
    let mut staged_order = VecDeque::with_capacity(events.len());
    let mut store_order_start = 0usize;
    let mut retained_len = store.event_order.len();

    for event in events {
        event.validate()?;
        let retained_in_store =
            store.events.contains_key(&event.id) && !pruned_store_ids.contains(&event.id);
        if retained_in_store || staged_ids.contains(&event.id) {
            return already_exists("event", event.id.to_string());
        }

        staged_ids.insert(event.id);
        staged_order.push_back(event.id);
        retained_len = retained_len.saturating_add(1);

        if let Some(max_events) = store.max_events {
            while retained_len > max_events {
                if let Some(pruned) = store.event_order.get(store_order_start).copied() {
                    store_order_start += 1;
                    pruned_store_ids.insert(pruned);
                } else {
                    let pruned = staged_order
                        .pop_front()
                        .expect("effective event order must contain the excess event");
                    staged_ids.remove(&pruned);
                }
                retained_len -= 1;
            }
        }
    }

    Ok(())
}

fn validate_modules(store: &InMemoryStore, modules: &[ModuleRecord]) -> MResult<Vec<bool>> {
    let mut staged_by_id = HashMap::<ModuleId, &str>::with_capacity(modules.len());
    let mut staged_by_name = HashMap::<&str, ModuleId>::with_capacity(modules.len());
    let mut inserts = Vec::with_capacity(modules.len());

    for module in modules {
        module.validate()?;
        if module.id != module_id(&module.name) {
            return Err(MechError::new(
                InvalidStoreRecordError {
                    field: "module.id",
                    reason: "module ID does not match its canonical URI",
                },
                None,
            ));
        }

        if let Some(existing_name) = staged_by_id.get(&module.id) {
            if *existing_name == module.name {
                inserts.push(false);
                continue;
            }
            return invalid_module_id_conflict();
        }
        if let Some(existing) = store.modules.get(&module.id) {
            if existing.name == module.name {
                inserts.push(false);
                continue;
            }
            return invalid_module_id_conflict();
        }

        if let Some(existing_id) = staged_by_name
            .get(module.name.as_str())
            .copied()
            .or_else(|| store.modules_by_name.get(&module.name).copied())
        {
            return Err(MechError::new(
                InvalidStoreRecordError {
                    field: "module.name",
                    reason: if existing_id == module.id {
                        "module name index is missing its primary record"
                    } else {
                        "canonical URI maps to another module ID"
                    },
                },
                None,
            ));
        }

        staged_by_id.insert(module.id, module.name.as_str());
        staged_by_name.insert(module.name.as_str(), module.id);
        inserts.push(true);
    }

    Ok(inserts)
}

fn validate_module_versions(
    store: &InMemoryStore,
    modules: &[ModuleRecord],
    versions: &[ModuleVersionRecord],
    module_inserts: &[bool],
) -> MResult<Vec<bool>> {
    let staged_modules = modules
        .iter()
        .zip(module_inserts)
        .filter_map(|(module, inserts)| inserts.then_some(module.id))
        .collect::<HashSet<_>>();
    let mut staged_versions = HashMap::<ModuleVersionId, &ModuleVersionRecord>::new();
    let mut inserts = Vec::with_capacity(versions.len());

    for version in versions {
        version.validate()?;
        version.validate_import_edges()?;
        if !store.modules.contains_key(&version.module) && !staged_modules.contains(&version.module)
        {
            return not_found("module", version.module.to_string());
        }
        for dependency in version
            .dependencies
            .iter()
            .chain(version.import_edges.iter().map(|edge| &edge.dependency))
        {
            if !store.module_versions.contains_key(dependency)
                && !staged_versions.contains_key(dependency)
            {
                return not_found("module_version", dependency.to_string());
            }
        }

        if let Some(existing) = staged_versions
            .get(&version.id)
            .copied()
            .or_else(|| store.module_versions.get(&version.id))
        {
            if existing == version {
                inserts.push(false);
                continue;
            }
            return Err(MechError::new(
                InvalidStoreRecordError {
                    field: "module_version.id",
                    reason: "version ID maps to different contents",
                },
                None,
            ));
        }

        staged_versions.insert(version.id, version);
        inserts.push(true);
    }

    Ok(inserts)
}

fn invalid_module_id_conflict<T>() -> MResult<T> {
    Err(MechError::new(
        InvalidStoreRecordError {
            field: "module.id",
            reason: "module ID maps to another canonical URI",
        },
        None,
    ))
}

fn already_exists<T>(record_type: &'static str, id: String) -> MResult<T> {
    Err(MechError::new(
        StoreRecordAlreadyExistsError { record_type, id },
        None,
    ))
}

fn not_found<T>(record_type: &'static str, id: String) -> MResult<T> {
    Err(MechError::new(
        StoreRecordNotFoundError { record_type, id },
        None,
    ))
}
