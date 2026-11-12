//! Diagnostic native contract over the SAME checked numeric computation plan.
//! No ordinary entry selects this contract. Its compact typed boundary omits
//! exact resource/recovery state; source arithmetic and safe access remain.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::compiler::{backend, offset};
use crate::native_artifact::program::prepare::{readonly_numeric::Proof, tail::Region};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use std::collections::{BTreeMap, BTreeSet};
#[path = "minimal_liveness.rs"]
mod liveness;
pub(in crate::native_artifact) const BOUNDS: u64 = 0x19301;
pub(in crate::native_artifact) const NEEDS_GROWTH: u64 = 0x19302;
pub(in crate::native_artifact) const UNSUPPORTED: u64 = 0x19303;

pub(in crate::native_artifact) fn input_words(
    proof: &Proof,
    carrier: u32,
) -> Result<Vec<usize>, NativeCompileError> {
    let mut out = proof
        .frame(carrier)
        .ok_or_else(|| backend("native input lacks carrier"))?
        .scalar_paths
        .iter()
        .map(|(_, word)| *word)
        .collect::<Vec<_>>();
    for (array, _) in proof.arrays.iter().enumerate() {
        out.extend(proof.view(array)..proof.view(array) + 4);
        out.extend(proof.capacity(array));
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}
pub(in crate::native_artifact) fn output_words(
    proof: &Proof,
) -> Result<Vec<usize>, NativeCompileError> {
    let mut out = Vec::new();
    if let Some(array) = proof.result.direct_array() {
        out.extend(proof.view(array)..proof.view(array) + 4);
        out.extend(proof.capacity(array));
    } else if let Some(frame) = proof.result.frame() {
        let recipe = proof
            .frames
            .iter()
            .find(|f| f.value == frame)
            .ok_or_else(|| backend("native result lost frame"))?;
        let mut scalars = recipe.scalar_paths.iter();
        for array in &recipe.arrays {
            if let Some(array) = array {
                out.extend(proof.view(*array)..proof.view(*array) + 4);
                out.extend(proof.capacity(*array));
            } else {
                let (_, word) = scalars
                    .next()
                    .ok_or_else(|| backend("native result lost scalar path"))?;
                out.push(*word);
            }
        }
        if scalars.next().is_some() {
            return Err(backend("native result has unbound scalar paths"));
        }
    } else {
        return Err(backend(
            "diagnostic native result requires current frame/array recipe",
        ));
    }
    Ok(out)
}
fn constants(code: CodePlan<'_>) -> Result<BTreeMap<usize, u64>, NativeCompileError> {
    let mut result = BTreeMap::new();
    for index in 0..code.constant_count() {
        let (value, word) = code.constant(index)?;
        if let Some(word) = word {
            result.insert(value.get() as usize, word);
        }
    }
    if let CodePlan::Program(p) = code {
        for (target, values) in p.metadata().scalar_aggregates.constants() {
            let (base, count) = p
                .metadata()
                .scalar_aggregates
                .slot(*target)
                .ok_or_else(|| backend("native aggregate constant lacks words"))?;
            if count != values.len() {
                return Err(backend("native aggregate constant width differs"));
            }
            for (index, &word) in values.iter().enumerate() {
                result.insert(base + index, word);
            }
        }
    }
    Ok(result)
}
pub(in crate::native_artifact) struct Boundary<'a> {
    storage: Option<&'a crate::native_artifact::program::compact::arrays::Arrays>,
    owners: Vec<Option<cranelift_frontend::Variable>>,
    fixed_capacities: Vec<Option<Value>>,
    arrays: usize,
    no_growth: Option<&'a crate::native_artifact::program::compact::NoGrowth>,
    pub input: Value,
    pub output: Value,
    pub entry: Vec<usize>,
    pub result: Vec<usize>,
    pub shadow: usize,
    pub exit: usize,
    handoffs: BTreeMap<usize, Vec<(usize, usize)>>,
}
impl<'a> Boundary<'a> {
    pub fn new(
        b: &mut FunctionBuilder<'_>,
        input: Value,
        output: Value,
        region: &Region,
        storage: Option<&'a crate::native_artifact::program::compact::arrays::Arrays>,
        private: Option<&crate::native_artifact::program::compact::PrivatePhase>,
        no_growth: Option<&'a crate::native_artifact::program::compact::NoGrowth>,
    ) -> Result<Self, NativeCompileError> {
        let proof = region
            .numeric
            .as_ref()
            .ok_or_else(|| backend("native computation requires numeric proof"))?;
        // A concrete compact caller can also enter a complete single-loop
        // proof. Standalone diagnostics retain their existing nested contract.
        let private = private.filter(|contract| storage.is_some() && contract.permits(proof));
        if private.is_none()
            && (!proof.exits.is_empty()
                || proof
                    .phase
                    .as_ref()
                    .map_or(storage.is_none(), |p| p.checked_children))
        {
            return Err(backend(
                "diagnostic native computation requires a complete synchronous phase",
            ));
        }
        let entry = input_words(proof, region.tail.carrier().get())?;
        let mut ordinal = 0;
        let owners = if storage.is_some() {
            proof
                .arrays
                .iter()
                .map(|array| {
                    if !array.writable {
                        return Ok(None);
                    }
                    let owner = b.declare_var(types::I64);
                    let value = b.ins().load(
                        types::I64,
                        MemFlags::new(),
                        input,
                        offset(entry.len() + ordinal)?,
                    );
                    ordinal += 1;
                    b.def_var(owner, value);
                    Ok(Some(owner))
                })
                .collect::<Result<Vec<_>, NativeCompileError>>()?
        } else {
            Vec::new()
        };
        // Fixed indexed updates cannot change backing capacity. Preserve its
        // actual typed entry value without inventing append coordinates.
        let mut fixed = 0;
        let fixed_capacities = proof
            .arrays
            .iter()
            .enumerate()
            .map(|(array, definition)| {
                if storage.is_none() || !definition.writable || proof.capacity(array).is_some() {
                    return Ok(None);
                }
                let value = b.ins().load(
                    types::I64,
                    MemFlags::new(),
                    input,
                    offset(entry.len() + ordinal + fixed)?,
                );
                fixed += 1;
                Ok(Some(value))
            })
            .collect::<Result<Vec<_>, NativeCompileError>>()?;
        Ok(Self {
            storage,
            owners,
            fixed_capacities,
            arrays: proof.arrays.len(),
            no_growth,
            input,
            output,
            entry: input_words(proof, region.tail.carrier().get())?,
            result: output_words(proof)?,
            shadow: region
                .shadow
                .ok_or_else(|| backend("native plan lacks logical field coordinates"))?,
            exit: region.tail.exit() as usize,
            handoffs: private
                .map(|contract| contract.exports(region.shadow.unwrap()))
                .unwrap_or_default(),
        })
    }
    pub fn words(
        &self,
        b: &mut FunctionBuilder<'_>,
        code: CodePlan<'_>,
        proof: &Proof,
        header: usize,
        slots: BTreeSet<usize>,
    ) -> Result<Words, NativeCompileError> {
        let constants = constants(code)?;
        let inputs = self
            .entry
            .iter()
            .map(|&word| self.shadow + word)
            .collect::<BTreeSet<_>>();
        let known = constants
            .keys()
            .copied()
            .chain(inputs.iter().copied())
            .collect();
        let required = liveness::required(
            code,
            proof,
            self.shadow,
            header,
            self.exit,
            &self.result,
            &self.handoffs,
        )?;
        let missing = required.difference(&known).copied().collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(backend(format!(
                "native entry has unknown initial scalar words: {missing:?}"
            )));
        }
        let zero = b.ins().iconst(types::I64, 0);
        Words::new_with_regions_and_initial(
            b,
            zero,
            code,
            slots,
            |b, slot| {
                let value = if let Some(at) = slot
                    .checked_sub(self.shadow)
                    .and_then(|word| self.entry.iter().position(|&v| v == word))
                {
                    b.ins()
                        .load(types::I64, MemFlags::new(), self.input, offset(at)?)
                } else if let Some(&value) = constants.get(&slot) {
                    b.ins().iconst(types::I64, value as i64)
                } else if proof.writes()
                    && (0..proof.arrays.len()).any(|a| self.shadow + proof.private_flag(a) == slot)
                {
                    b.ins().iconst(types::I64, 1)
                } else {
                    zero
                };
                Ok(Some(value))
            },
            |_| MemFlags::new(),
        )
    }
    pub fn handoff(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        site: usize,
    ) -> Result<Option<usize>, NativeCompileError> {
        let Some(exports) = self.handoffs.get(&site) else {
            return Ok(None);
        };
        for &(at, source) in exports {
            let value = words.read(b, source);
            b.ins()
                .store(MemFlags::new(), value, self.output, offset(at)?);
        }
        let status = b.ins().iconst(
            types::I64,
            crate::native_artifact::program::compact::PRIVATE_HANDOFF as i64,
        );
        let at = b.ins().iconst(types::I64, site as i64);
        b.ins().return_(&[status, at]);
        Ok(Some(exports.len()))
    }
    pub fn reject(&self, b: &mut FunctionBuilder<'_>, status: u64, error: Block) {
        let status = b.ins().iconst(types::I64, status as i64);
        b.ins().jump(error, &[status.into()]);
    }
    pub fn capacity(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        proof: &Proof,
        array: usize,
        maximum: usize,
        error: Block,
        cold_boundaries: &mut BTreeSet<Block>,
    ) -> Result<(), NativeCompileError> {
        if self
            .no_growth
            .is_some_and(|contract| contract.proves_append(array, maximum))
        {
            return Ok(());
        }
        let length = words.read(b, self.shadow + proof.view(array) + 1);
        let capacity = words.read(
            b,
            self.shadow
                + proof
                    .capacity(array)
                    .ok_or_else(|| backend("native append lacks capacity"))?,
        );
        let valid = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, length, capacity);
        let available = b.ins().isub(capacity, length);
        let fits = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, available, maximum as i64);
        let enough = b.ins().band(valid, fits);
        let good = b.create_block();
        let bad = b.create_block();
        b.set_cold_block(bad);
        b.ins().brif(enough, good, &[], bad, &[]);
        b.switch_to_block(bad);
        if let Some(storage) = self.storage {
            // Real growth is a cold concrete-allocation boundary. The ordinary
            // funded successful path retains its existing no-general-call check.
            b.set_cold_block(bad);
            cold_boundaries.insert(bad);
            if maximum != 1 {
                return Err(backend(
                    "compact numeric adapter requires unit append growth",
                ));
            }
            let at = self.shadow + proof.view(array);
            let owner = b.use_var(self.owners[array].expect("writable compact owner"));
            let view = [
                words.read(b, at),
                length,
                capacity,
                words.read(b, at + 2),
                words.read(b, at + 3),
                owner,
            ];
            let result = storage.grow_numeric(b, &view, proof.arrays[array].width, error)?;
            for (source, destination) in [(0, at), (1, at + 1), (3, at + 2), (4, at + 3)] {
                words.write(b, destination, result[source]);
            }
            words.write(b, self.shadow + proof.capacity(array).unwrap(), result[2]);
            b.def_var(
                self.owners[array].expect("writable compact owner"),
                result[5],
            );
            b.ins().jump(good, &[]);
        } else {
            self.reject(b, NEEDS_GROWTH, error);
        }
        b.switch_to_block(good);
        Ok(())
    }
    pub fn origin(
        &self,
        b: &mut FunctionBuilder<'_>,
        site: usize,
    ) -> Result<(), NativeCompileError> {
        let site = b.ins().iconst(types::I64, site as i64);
        b.ins().store(MemFlags::new(), site, self.output, 0);
        Ok(())
    }
    pub fn finish(
        &self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        proof: &Proof,
    ) -> Result<(), NativeCompileError> {
        for (at, &word) in self.result.iter().enumerate() {
            let value = words.read(b, self.shadow + word);
            b.ins()
                .store(MemFlags::new(), value, self.output, offset(1 + at)?);
        }
        if self.storage.is_some() {
            let mut ordinal = 0;
            for array in 0..self.arrays {
                let Some(owner) = self.owners[array] else {
                    continue;
                };
                let view = self.shadow + proof.view(array);
                let capacity = if let Some(capacity) = proof.capacity(array) {
                    words.read(b, self.shadow + capacity)
                } else {
                    self.fixed_capacities[array]
                        .ok_or_else(|| backend("compact fixed result lacks actual capacity"))?
                };
                let current = [
                    words.read(b, view),
                    words.read(b, view + 1),
                    capacity,
                    words.read(b, view + 2),
                    words.read(b, view + 3),
                    b.use_var(owner),
                ];
                for (lane, value) in current.into_iter().enumerate() {
                    b.ins().store(
                        MemFlags::new(),
                        value,
                        self.output,
                        offset(1 + self.result.len() + ordinal * 6 + lane)?,
                    );
                }
                ordinal += 1;
            }
        }
        Ok(())
    }
}
