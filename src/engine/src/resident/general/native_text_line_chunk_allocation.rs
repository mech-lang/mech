//! Admitted backing, exact capacities, cumulative work, and physical drop order.
use super::{InvokeError, NativeLibraryWorkBudget};
use mech_core::{
    ManagedMemoryBudget, ManagedMemoryReservation, MemoryBudgetDimension as Dimension,
    MemoryBudgetLimits, ResidentKernelError, ResidentResourcePhase,
};
use std::{cell::Cell, rc::Rc};

pub(super) struct Account {
    memory: ManagedMemoryBudget,
    work: NativeLibraryWorkBudget,
    used: Cell<u64>,
    peak: Cell<u64>,
    spent: Cell<u64>,
    temporary: u64,
    compute: u64,
}
fn limit(dimension: Dimension, required: u64, limit: u64) -> Result<(), InvokeError> {
    if required > limit {
        return Err(ResidentKernelError::ResourceLimit {
            phase: ResidentResourcePhase::Admission,
            dimension,
            required,
            limit,
        }
        .into());
    }
    Ok(())
}
impl Account {
    pub fn new(
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        fixed: u64,
        lookup_work: u64,
    ) -> Result<(Rc<Self>, ManagedMemoryReservation), InvokeError> {
        let temporary = limits
            .max_temporary_bytes
            .ok_or(InvokeError::InvalidInput)?;
        let compute = limits.max_compute_work.ok_or(InvokeError::InvalidInput)?;
        let bytes = fixed
            .checked_add(core::mem::size_of::<Self>() as u64)
            .and_then(|bytes| bytes.checked_add(512))
            .ok_or(InvokeError::InvalidInput)?;
        limit(Dimension::TemporaryBytes, bytes, temporary)?;
        let debit = bytes
            .checked_mul(8)
            .and_then(|work| work.checked_add(128))
            .and_then(|work| work.checked_add(lookup_work))
            .ok_or(InvokeError::InvalidInput)?;
        limit(Dimension::ComputeWork, debit, compute)?;
        work.charge(debit, 0)?;
        let memory = work.memory_budget();
        let fixed = memory.reserve_capacity(bytes)?;
        Ok((
            Rc::new(Self {
                memory,
                work: work.clone(),
                used: Cell::new(bytes),
                peak: Cell::new(bytes),
                spent: Cell::new(debit),
                temporary,
                compute,
            }),
            fixed,
        ))
    }
    pub fn used(&self) -> u64 {
        self.used.get()
    }
    pub fn shares_work(&self, work: &NativeLibraryWorkBudget) -> bool {
        self.work.shares_account(work)
    }
    pub fn shares_memory(&self, memory: &ManagedMemoryBudget) -> bool {
        self.memory == *memory
    }
    pub fn peak(&self) -> u64 {
        self.peak.get()
    }
    pub fn spent(&self) -> u64 {
        self.spent.get()
    }
    pub fn debit(&self, work: u64) -> Result<(), InvokeError> {
        let spent = self
            .spent
            .get()
            .checked_add(work)
            .ok_or(InvokeError::InvalidInput)?;
        limit(Dimension::ComputeWork, spent, self.compute)?;
        self.work.charge(work, 0)?;
        self.spent.set(spent);
        Ok(())
    }
    pub fn reserve(account: &Rc<Self>, bytes: u64) -> Result<Claim, InvokeError> {
        let used = account
            .used
            .get()
            .checked_add(bytes)
            .ok_or(InvokeError::InvalidInput)?;
        limit(Dimension::TemporaryBytes, used, account.temporary)?;
        account.debit(
            bytes
                .checked_mul(8)
                .and_then(|work| work.checked_add(128))
                .ok_or(InvokeError::InvalidInput)?,
        )?;
        let reservation = account.memory.reserve_capacity(bytes)?;
        account.used.set(used);
        account.peak.set(account.peak.get().max(used));
        Ok(Claim {
            reservation: Some(reservation),
            account: account.clone(),
            bytes,
        })
    }
}
pub(super) struct Claim {
    reservation: Option<ManagedMemoryReservation>,
    account: Rc<Account>,
    bytes: u64,
}
impl Drop for Claim {
    fn drop(&mut self) {
        // Container fields dropped their data first. Retire the original
        // managed claim before reducing the private capacity observation.
        drop(self.reservation.take());
        self.account.used.set(self.account.used.get() - self.bytes);
    }
}
pub(super) struct Bytes {
    pub text: String,
    _claim: Option<Claim>,
}
impl Bytes {
    pub fn copy(text: &str, account: &Rc<Account>) -> Result<Self, InvokeError> {
        let claim = if text.is_empty() {
            None
        } else {
            Some(Account::reserve(account, text.len() as u64)?)
        };
        let mut owned = String::new();
        owned
            .try_reserve_exact(text.len())
            .map_err(|_| InvokeError::InvalidInput)?;
        if owned.capacity() != text.len() {
            return Err(InvokeError::InvalidInput);
        }
        owned.push_str(text);
        Ok(Self {
            text: owned,
            _claim: claim,
        })
    }
    pub fn append(
        &mut self,
        text: &str,
        input_limit: usize,
        account: &Rc<Account>,
    ) -> Result<(), InvokeError> {
        let needed = self
            .text
            .len()
            .checked_add(text.len())
            .ok_or(InvokeError::InvalidInput)?;
        if needed > self.text.capacity() {
            let capacity = self
                .text
                .capacity()
                .checked_mul(2)
                .unwrap_or(input_limit)
                .max(needed)
                .min(input_limit);
            if capacity < needed {
                return Err(InvokeError::InvalidInput);
            }
            let claim = Account::reserve(account, capacity as u64)?;
            let mut owned = String::new();
            owned
                .try_reserve_exact(capacity)
                .map_err(|_| InvokeError::InvalidInput)?;
            if owned.capacity() != capacity {
                return Err(InvokeError::InvalidInput);
            }
            owned.push_str(&self.text);
            owned.push_str(text);
            *self = Self {
                text: owned,
                _claim: Some(claim),
            };
        } else {
            self.text.push_str(text);
        }
        Ok(())
    }
}
pub(super) struct Offsets {
    pub values: Vec<u32>,
    _claim: Claim,
}
impl Offsets {
    pub fn new(text: &str, count: usize, account: &Rc<Account>) -> Result<Self, InvokeError> {
        let bytes = count
            .checked_mul(core::mem::size_of::<u32>())
            .ok_or(InvokeError::InvalidInput)?;
        let claim = Account::reserve(account, bytes as u64)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| InvokeError::InvalidInput)?;
        if values.capacity() != count {
            return Err(InvokeError::InvalidInput);
        }
        // Use the standard String delimiter search (including its optimized
        // ASCII search), not a scalar Rust branch for every input byte.
        for (index, _) in text.match_indices('\n') {
            values.push(u32::try_from(index).map_err(|_| InvokeError::InvalidInput)?);
        }
        if values.len() != count {
            return Err(InvokeError::InvalidInput);
        }
        Ok(Self {
            values,
            _claim: claim,
        })
    }
}
