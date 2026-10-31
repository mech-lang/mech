//! A selected non-Value String projection's honest current demand, not a
//! fabricated TurnMemoryPlan or authority to execute an artifact operation.
use super::*;

pub(in crate::resident) struct NativeProjectionIndexContext<'a> {
    work: &'a NativeLibraryWorkBudget,
    limits: MemoryBudgetLimits,
    prior: ResourceDemand,
    start: (u64, u64),
    // Move the actual failed reservation error through this fixed stack slot;
    // no fabricated payload-admission/TurnMemoryPlan scope is needed.
    memory_error: std::cell::RefCell<Option<mech_core::MemoryRuntimeError>>,
}

impl<'a> NativeProjectionIndexContext<'a> {
    // The prepared call validates its exact selected identity, finite policy,
    // source/schema admission and per-site owning account BEFORE construction.
    pub(in crate::resident) fn new(
        work: &'a NativeLibraryWorkBudget,
        limits: MemoryBudgetLimits,
        prior: ResourceDemand,
    ) -> Self {
        Self {
            work,
            limits,
            prior,
            start: work.used(),
            memory_error: Default::default(),
        }
    }

    pub(in crate::resident) fn with_added_capacity(
        &self,
        bytes: u64,
        nodes: u64,
    ) -> Result<Self, ResidentKernelError> {
        let mut prior = self.prior;
        prior.turn_peak_bytes = prior
            .turn_peak_bytes
            .checked_add(bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        prior.retained_nodes = prior
            .retained_nodes
            .checked_add(nodes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        Ok(Self {
            work: self.work,
            limits: self.limits,
            prior,
            start: self.start,
            memory_error: Default::default(),
        })
    }

    pub(in crate::resident) fn take_memory_error(&self) -> Option<mech_core::MemoryRuntimeError> {
        self.memory_error.borrow_mut().take()
    }

    fn check(
        &self,
        compute: u64,
        comparison: u64,
        bytes: u64,
        nodes: u64,
    ) -> Result<(), ResidentKernelError> {
        let used = self.work.used();
        let mut demand = self.prior;
        demand.work.compute = demand
            .work
            .compute
            .checked_add(
                used.0
                    .checked_sub(self.start.0)
                    .ok_or(ResidentKernelError::InvalidInput)?,
            )
            .and_then(|n| n.checked_add(compute))
            .ok_or(ResidentKernelError::InvalidShape)?;
        demand.work.comparison = demand
            .work
            .comparison
            .checked_add(
                used.1
                    .checked_sub(self.start.1)
                    .ok_or(ResidentKernelError::InvalidInput)?,
            )
            .and_then(|n| n.checked_add(comparison))
            .ok_or(ResidentKernelError::InvalidShape)?;
        demand.turn_peak_bytes = demand
            .turn_peak_bytes
            .checked_add(bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        demand.retained_nodes = demand
            .retained_nodes
            .checked_add(nodes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        if let Some(violation) = mech_core::first_call_memory_budget_violation(
            mech_core::MemoryObjectOwner::DirectCallPort {
                call: 0,
                direction: mech_core::PortDirection::Input,
                port: 0,
            },
            demand,
            0,
            0,
            self.limits,
        ) {
            return Err(resource_budget_error(&violation));
        }
        Ok(())
    }

    // Validate original cumulative selected-sink limits before every actual
    // debit. A successful debit goes to the SAME invocation ledger; this
    // context never refunds, refills, or maintains a parallel work account.
    pub(in crate::resident) fn charge(
        &self,
        compute: u64,
        comparison: u64,
    ) -> Result<(), ResidentKernelError> {
        self.check(compute, comparison, 0, 0)?;
        self.work.charge(compute, comparison)
    }

    /// Only a completely funded original prefix may skip its intermediate
    /// checks. Include this projection's prior demand before the same scoped
    /// account checks; every declined preview leaves all observations intact.
    pub(in crate::resident) fn try_charge_prefix(&self, compute: u64, comparison: u64) -> bool {
        self.check(compute, comparison, 0, 0).is_ok()
            && self.work.try_charge_scoped_prefix(compute, comparison)
    }

    pub(in crate::resident) fn check_capacity(
        &self,
        bytes: u64,
        nodes: u64,
    ) -> Result<(), ResidentKernelError> {
        self.check(0, 0, bytes, nodes)
    }

    pub(in crate::resident) fn prepare_index(
        &self,
        slot: &crate::resident::text_index::snapshot::Slot,
        source: &Value,
    ) -> Result<crate::resident::text_index::IndexUse, ResidentKernelError> {
        // A dense lexical index has a different live-domain accounting scope;
        // do not borrow its authority or invent a fixed plan for this route.
        if crate::resident::text_index::has_index_scope()
            || crate::resident::text_index::live_demand() != (0, 0)
        {
            return Err(ResidentKernelError::InvalidInput);
        }
        // Hits also re-admit exact current owner coexistence and cached index
        // residency. The weak identity is cache metadata, never a source pin.
        self.check_capacity(slot.bytes(), slot.nodes())?;
        slot.prepare_with_charge(
            source,
            self.work,
            |compute, comparison| self.charge(compute, comparison),
            |bytes, nodes| self.check_capacity(bytes, nodes),
            |error| {
                *self.memory_error.borrow_mut() = Some(error);
                ResidentKernelError::InvalidShape
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        ManagedMemoryBudget, MemoryBudgetDimension, SchemaDraft, SchemaTableBuilder,
        ValueDataDraft, ValueDraft, snapshot::SnapshotValidationContext,
    };

    fn source(text: &str, memory: &ManagedMemoryBudget) -> Value {
        let mut builder = SchemaTableBuilder::new();
        let handle = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::String,
                    dimension_parameters: Box::new([]),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = builder.finish().unwrap();
        let value = ValueDraft {
            schema: build.resolve(handle).unwrap(),
            shape_values: Box::new([]),
            data: ValueDataDraft::String(text.into()),
        }
        .finalize(&SnapshotValidationContext::new(&build.table))
        .unwrap();
        let mut reservation = memory
            .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
            .unwrap();
        value.into_memory_budget(&mut reservation).unwrap()
    }

    #[test]
    fn cumulative_component_preflight_precedes_same_ledger_debit_and_never_refunds() {
        let memory = ManagedMemoryBudget::new(1 << 20);
        let work = NativeLibraryWorkBudget::new(1000, 1000, &memory).unwrap();
        let limits = MemoryBudgetLimits {
            max_compute_work: Some(10),
            max_comparison_work: Some(5),
            max_temporary_bytes: Some(256),
            max_retained_nodes: Some(8),
            max_cloned_bytes: Some(3),
            ..MemoryBudgetLimits::default()
        };
        let mut prior = ResourceDemand {
            turn_peak_bytes: 11,
            retained_nodes: 2,
            cloned_bytes: 3,
            ..ResourceDemand::default()
        };
        prior.work.compute = 7;
        prior.work.comparison = 2;
        let context = NativeProjectionIndexContext::new(&work, limits, prior);
        context.charge(3, 3).unwrap();
        assert_eq!(work.used(), (3, 3));
        assert!(matches!(
            context.charge(1, 0),
            Err(ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::ComputeWork,
                required: 11,
                limit: 10,
                ..
            })
        ));
        assert_eq!(work.used(), (3, 3));
        context.check_capacity(245, 6).unwrap();
        assert!(context.check_capacity(246, 6).is_err());
        assert!(context.check_capacity(245, 7).is_err());
        let retained = context.with_added_capacity(32, 1).unwrap();
        retained.check_capacity(213, 5).unwrap();
        assert!(retained.check_capacity(214, 5).is_err());
        assert_eq!(work.used(), (3, 3));
        drop(work);
        assert_eq!(memory.used_bytes(), 0);
    }

    #[test]
    fn actual_index_reservation_error_survives_without_a_payload_plan_and_recovers() {
        let memory = ManagedMemoryBudget::new(8 << 20);
        let work = NativeLibraryWorkBudget::new(100_000_000, 1_000_000, &memory).unwrap();
        let source = source("e\u{301}🧑‍🚀\r\n", &memory);
        let slot = crate::resident::text_index::snapshot::Slot::default();
        let limits = MemoryBudgetLimits {
            max_temporary_bytes: Some(8 << 20),
            max_retained_nodes: Some(100),
            max_compute_work: Some(100_000_000),
            max_comparison_work: Some(1_000_000),
            ..MemoryBudgetLimits::default()
        };
        let prior = ResourceDemand {
            turn_peak_bytes: source.memory_budget_retained_bytes(&memory).unwrap(),
            retained_nodes: 2,
            ..ResourceDemand::default()
        };
        let context = NativeProjectionIndexContext::new(&work, limits, prior);
        let baseline = memory.used_bytes();
        let ballast = memory
            .reserve_capacity(memory.limit_bytes() - baseline)
            .unwrap();
        let before = work.used();
        assert!(context.prepare_index(&slot, &source).is_err());
        assert!(matches!(
            context.take_memory_error(),
            Some(mech_core::MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested,
                limit,
            }) if requested > limit
        ));
        assert_eq!((slot.bytes(), slot.nodes()), (0, 0));
        assert!(work.used().0 > before.0);
        assert_eq!(memory.used_bytes(), memory.limit_bytes());
        drop(ballast);
        assert_eq!(memory.used_bytes(), baseline);
        let index = context.prepare_index(&slot, &source).unwrap();
        assert_eq!(index.len(), 3);
        assert!(context.take_memory_error().is_none());
        drop((index, slot, source, work));
        assert_eq!(memory.used_bytes(), 0);
    }

    #[test]
    fn cached_index_hit_readmits_full_current_owner_and_exact_index_coexistence() {
        let memory = ManagedMemoryBudget::new(8 << 20);
        let work = NativeLibraryWorkBudget::new(100_000_000, 1_000_000, &memory).unwrap();
        let source = source("e\u{301}🧑‍🚀\r\n🇺🇸\0é", &memory);
        let slot = crate::resident::text_index::snapshot::Slot::default();
        let built = slot.prepare(&source, &work, |_, _| Ok(())).unwrap();
        assert_eq!(built.len(), 6);
        drop(built);
        let baseline = memory.used_bytes();
        let owner = source.memory_budget_retained_bytes(&memory).unwrap();
        let prior = ResourceDemand {
            turn_peak_bytes: owner + 64,
            retained_nodes: 2,
            ..ResourceDemand::default()
        };
        let mut limits = MemoryBudgetLimits {
            max_temporary_bytes: Some(prior.turn_peak_bytes + slot.bytes() - 1),
            max_retained_nodes: Some(2 + slot.nodes()),
            max_compute_work: Some(100_000_000),
            max_comparison_work: Some(1_000_000),
            ..MemoryBudgetLimits::default()
        };
        let before = work.used();
        let denied = NativeProjectionIndexContext::new(&work, limits, prior);
        assert!(matches!(
            denied.prepare_index(&slot, &source),
            Err(ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::TemporaryBytes,
                ..
            })
        ));
        assert_eq!(work.used(), before);
        assert_eq!(memory.used_bytes(), baseline);
        limits.max_temporary_bytes = Some(prior.turn_peak_bytes + slot.bytes());
        limits.max_retained_nodes = Some(2 + slot.nodes() - 1);
        let denied = NativeProjectionIndexContext::new(&work, limits, prior);
        assert!(matches!(
            denied.prepare_index(&slot, &source),
            Err(ResidentKernelError::ResourceLimit {
                dimension: MemoryBudgetDimension::RetainedNodes,
                ..
            })
        ));
        limits.max_retained_nodes = Some(2 + slot.nodes());
        let exact = NativeProjectionIndexContext::new(&work, limits, prior);
        let hit = exact.prepare_index(&slot, &source).unwrap();
        assert_eq!(hit.byte_range(0), Some(0..3));
        assert_eq!(memory.used_bytes(), baseline);
        assert!(work.used().0 > before.0);
        drop((hit, slot, source, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}
