//! Persisted native code plus the layout required to call it without a compiler.
//! Hashes detect corruption/mix-ups; they do not authenticate executable code.

use super::{Compiled, Error, Interface, Kernel};
use crate::{BatchedExecutionError, BatchedIntegrityFault};
use mech_core::{CellSlotId, IntegrityConstraintId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

const FORMAT_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 256 * 1024 * 1024;
const MAX_BUFFER_ELEMENTS: usize = 256 * 1024 * 1024;
type NativeTurn =
    unsafe extern "C" fn(*const *const f32, *const *const f32, *const *mut f32, usize) -> u64;

fn error(message: impl Into<String>) -> Error {
    Error::Bundle(message.into())
}
fn io(error_value: impl std::fmt::Display) -> Error {
    error(error_value.to_string())
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    os: String,
    arch: String,
    pointer_width: u32,
    endian: String,
}
impl Target {
    fn current() -> Self {
        Self {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            pointer_width: usize::BITS,
            endian: if cfg!(target_endian = "little") {
                "little"
            } else {
                "big"
            }
            .into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum NativeBackend {
    Scalar,
    Simd,
}
impl NativeBackend {
    fn symbol(self) -> &'static [u8] {
        match self {
            Self::Scalar => b"mech_fixed_numeric_turn\0",
            Self::Simd => b"mech_fixed_numeric_simd_turn\0",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    name: String,
    slot: u32,
    width: usize,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    slot: u32,
    width: usize,
    initializer_bits: Vec<u32>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Constraint {
    id: u32,
    name: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    target: Target,
    backend: NativeBackend,
    library_file: String,
    library_sha256: String,
    instances: u32,
    inputs: Vec<Input>,
    states: Vec<State>,
    constraints: Vec<Constraint>,
    exports: BTreeMap<String, u32>,
    // IEEE-754 bits preserve non-finite values and signed zero exactly.
    initial_inputs: BTreeMap<String, Vec<u32>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format_version: u32,
    metadata_sha256: String,
    metadata: Metadata,
}

struct LoadedLibrary {
    _library: libloading::Library,
    turn: NativeTurn,
    path: PathBuf,
}
pub(super) struct NativeBundle {
    metadata: Arc<Metadata>,
    library: Arc<LoadedLibrary>,
}

fn library_file() -> &'static str {
    if cfg!(target_os = "macos") {
        "kernel.dylib"
    } else {
        "kernel.so"
    }
}

fn checked_length(width: usize, instances: u32) -> Result<usize, Error> {
    let count = width
        .checked_mul(instances as usize)
        .ok_or_else(|| error("buffer length overflow"))?;
    if width == 0 || count > MAX_BUFFER_ELEMENTS {
        return Err(error("empty or oversized native buffer"));
    }
    Ok(count)
}

impl Metadata {
    fn validate(&self) -> Result<(), Error> {
        if self.target != Target::current() {
            return Err(error("bundle target does not match this process"));
        }
        if self.library_file != library_file() {
            return Err(error("invalid native library filename"));
        }
        if self.instances == 0 || (self.backend == NativeBackend::Simd && self.instances % 4 != 0) {
            return Err(error("invalid native batch extent"));
        }
        if self.inputs.len() > 65536 || self.states.len() > 65536 || self.constraints.len() > 255 {
            return Err(error("native interface exceeds supported limits"));
        }
        let mut slots = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut total = 0usize;
        for input in &self.inputs {
            if input.name.is_empty() || !slots.insert(input.slot) || !names.insert(&input.name) {
                return Err(error("duplicate or invalid native input"));
            }
            let count = checked_length(input.width, self.instances)?;
            total = total
                .checked_add(count)
                .ok_or_else(|| error("buffer total overflow"))?;
            let values = self
                .initial_inputs
                .get(&input.name)
                .ok_or_else(|| error("missing initial input"))?;
            if values.len() != input.width && values.len() != count {
                return Err(error("invalid initial input length"));
            }
        }
        if self.initial_inputs.len() != self.inputs.len() {
            return Err(error("unexpected initial input"));
        }
        let mut states = BTreeMap::new();
        for state in &self.states {
            if !slots.insert(state.slot) || state.initializer_bits.len() != state.width {
                return Err(error("duplicate state or invalid initializer length"));
            }
            let count = checked_length(state.width, self.instances)?;
            total = total
                .checked_add(
                    count
                        .checked_mul(3)
                        .ok_or_else(|| error("state allocation overflow"))?,
                )
                .ok_or_else(|| error("buffer total overflow"))?;
            states.insert(state.slot, state.width);
        }
        if total > MAX_BUFFER_ELEMENTS {
            return Err(error("native session memory limit exceeded"));
        }
        for (name, slot) in &self.exports {
            if name.is_empty() || !states.contains_key(slot) {
                return Err(error("invalid named state export"));
            }
        }
        let mut ids = BTreeSet::new();
        for constraint in &self.constraints {
            if !ids.insert(constraint.id) || constraint.name.is_empty() {
                return Err(error("invalid integrity metadata"));
            }
        }
        Ok(())
    }

    fn interface(&self) -> Interface {
        Interface {
            input_widths: self
                .inputs
                .iter()
                .map(|input| (input.name.clone(), input.width))
                .collect(),
            states: self
                .exports
                .iter()
                .map(|(name, slot)| {
                    let width = self
                        .states
                        .iter()
                        .find(|state| state.slot == *slot)
                        .expect("validated export")
                        .width;
                    (name.clone(), (CellSlotId::new(*slot), width))
                })
                .collect(),
        }
    }
}

impl Kernel {
    /// Save an AOT library and its initial input/state/interface metadata.
    /// The directory must not exist. This saves a new-session initializer,
    /// not a checkpoint of any running session. JIT/evaluator kernels cannot be saved.
    pub fn save_bundle(&self, directory: impl AsRef<Path>) -> Result<(), Error> {
        let source_path = self
            .library_path()
            .ok_or_else(|| error("only AOT kernels can be saved"))?;
        let bytes = fs::read(source_path).map_err(io)?;
        let expected_hash = match &self.compiled {
            Compiled::Aot(artifact) => artifact.library_sha256(),
            Compiled::AotSimd(artifact) => artifact.library_sha256(),
            Compiled::Bundle(bundle) => &bundle.metadata.library_sha256,
            _ => return Err(error("only AOT kernels can be saved")),
        };
        if digest(&bytes) != expected_hash {
            return Err(error("native library changed after it was loaded"));
        }
        let metadata = match &self.compiled {
            Compiled::Bundle(bundle) => (*bundle.metadata).clone(),
            Compiled::Aot(_) | Compiled::AotSimd(_) => {
                let program = self.program.as_ref().expect("source-compiled AOT kernel");
                let initializers = program
                    .native_state_initializers()
                    .collect::<BTreeMap<_, _>>();
                Metadata {
                    target: Target::current(),
                    backend: if matches!(self.compiled, Compiled::AotSimd(_)) {
                        NativeBackend::Simd
                    } else {
                        NativeBackend::Scalar
                    },
                    library_file: library_file().into(),
                    library_sha256: digest(&bytes),
                    instances: self.instances(),
                    inputs: program
                        .native_input_layout()
                        .map(|(slot, name, width)| Input {
                            width,
                            name: name.into(),
                            slot: slot.get(),
                        })
                        .collect(),
                    states: program
                        .state_layout()
                        .map(|(slot, width)| State {
                            slot: slot.get(),
                            width,
                            initializer_bits: initializers[&slot]
                                .iter()
                                .map(|value| value.to_bits())
                                .collect(),
                        })
                        .collect(),
                    constraints: program
                        .named_integrity_constraints()
                        .map(|(id, name)| Constraint {
                            id: id.get(),
                            name: name.into(),
                        })
                        .collect(),
                    exports: self
                        .interface
                        .states
                        .iter()
                        .map(|(name, (slot, _))| (name.clone(), slot.get()))
                        .collect(),
                    initial_inputs: self
                        .initial_inputs
                        .iter()
                        .map(|(name, values)| {
                            (
                                name.clone(),
                                values.iter().map(|value| value.to_bits()).collect(),
                            )
                        })
                        .collect(),
                }
            }
            _ => return Err(error("only AOT kernels can be saved")),
        };
        metadata.validate()?;
        if digest(&bytes) != metadata.library_sha256 {
            return Err(error("native library changed since bundle loading"));
        }
        let metadata_bytes = serde_json::to_vec(&metadata).map_err(io)?;
        let manifest = Manifest {
            format_version: FORMAT_VERSION,
            metadata_sha256: digest(&metadata_bytes),
            metadata,
        };
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(io)?;
        if manifest_bytes.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(error("manifest exceeds size limit"));
        }
        let directory = directory.as_ref();
        fs::create_dir(directory).map_err(io)?;
        // Create-new avoids replacing a library or manifest in an existing artifact.
        let mut library = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(library_file()))
            .map_err(io)?;
        library.write_all(&bytes).map_err(io)?;
        library.sync_all().map_err(io)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("manifest.json"))
            .map_err(io)?;
        file.write_all(&manifest_bytes).map_err(io)?;
        file.sync_all().map_err(io)?;
        Ok(())
    }

    /// Load an AOT bundle without parsing source, lowering, compiling or linking.
    /// Target, interface limits, metadata digest and library digest are checked.
    /// These hashes detect accidental changes, not malicious native code.
    ///
    /// # Safety
    /// The caller must trust the entire bundle and its provenance: native library
    /// initializers can run during loading. Its executable code must implement the
    /// declared ABI and layouts. The bundle must not change during loading, and
    /// the native library must remain immutable while any kernel or session uses it.
    /// Only load artifacts produced by a compatible trusted Mech build
    /// on a CPU compatible with the emitting host. Hashes are not signatures.
    pub unsafe fn load_bundle(directory: impl AsRef<Path>) -> Result<Self, Error> {
        let directory = fs::canonicalize(directory).map_err(io)?;
        let manifest_path = directory.join("manifest.json");
        let file_metadata = fs::metadata(&manifest_path).map_err(io)?;
        if file_metadata.len() > MAX_MANIFEST_BYTES {
            return Err(error("manifest exceeds size limit"));
        }
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(manifest_path).map_err(io)?).map_err(io)?;
        if manifest.format_version != FORMAT_VERSION {
            return Err(error("unsupported bundle format version"));
        }
        let metadata_bytes = serde_json::to_vec(&manifest.metadata).map_err(io)?;
        if digest(&metadata_bytes) != manifest.metadata_sha256 {
            return Err(error("bundle metadata digest mismatch"));
        }
        manifest.metadata.validate()?;
        let path = directory.join(&manifest.metadata.library_file);
        if fs::symlink_metadata(&path)
            .map_err(io)?
            .file_type()
            .is_symlink()
        {
            return Err(error("native library must not be a symlink"));
        }
        let library_bytes = fs::read(&path).map_err(io)?;
        if digest(&library_bytes) != manifest.metadata.library_sha256 {
            return Err(error("native library digest mismatch"));
        }
        // SAFETY: caller guarantees trusted immutable native code and ABI identity.
        let library = unsafe { libloading::Library::new(&path) }.map_err(io)?;
        let turn = unsafe {
            *library
                .get::<NativeTurn>(manifest.metadata.backend.symbol())
                .map_err(io)?
        };
        let interface = Arc::new(manifest.metadata.interface());
        let initial_inputs = manifest
            .metadata
            .initial_inputs
            .iter()
            .map(|(name, bits)| {
                (
                    name.clone(),
                    bits.iter().copied().map(f32::from_bits).collect(),
                )
            })
            .collect();
        Ok(Self {
            program: None,
            compiled: Compiled::Bundle(NativeBundle {
                metadata: Arc::new(manifest.metadata),
                library: Arc::new(LoadedLibrary {
                    _library: library,
                    turn,
                    path,
                }),
            }),
            initial_inputs,
            interface,
        })
    }
}

impl NativeBundle {
    pub(super) fn instances(&self) -> u32 {
        self.metadata.instances
    }
    pub(super) fn path(&self) -> &Path {
        &self.library.path
    }
    pub(super) fn start(&self) -> Result<NativeSession, Error> {
        let mut inputs = BTreeMap::new();
        for input in &self.metadata.inputs {
            let values = self.metadata.initial_inputs[&input.name]
                .iter()
                .copied()
                .map(f32::from_bits)
                .collect::<Vec<_>>();
            inputs.insert(input.slot, self.metadata.pack(&values, input.width));
        }
        let state = self
            .metadata
            .states
            .iter()
            .map(|descriptor| {
                let values = descriptor
                    .initializer_bits
                    .iter()
                    .copied()
                    .cycle()
                    .take(descriptor.width * self.metadata.instances as usize)
                    .map(f32::from_bits)
                    .collect::<Vec<_>>();
                (CellSlotId::new(descriptor.slot), values)
            })
            .collect::<BTreeMap<_, _>>();
        let packed_state = self
            .metadata
            .states
            .iter()
            .map(|descriptor| {
                (
                    descriptor.slot,
                    self.metadata
                        .pack(&state[&CellSlotId::new(descriptor.slot)], descriptor.width),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let candidate = packed_state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        Ok(NativeSession {
            metadata: Arc::clone(&self.metadata),
            library: Arc::clone(&self.library),
            inputs,
            state,
            packed_state,
            candidate,
            attempted_turns: 0,
        })
    }
}

impl Metadata {
    fn pack(&self, values: &[f32], width: usize) -> Vec<f32> {
        let expanded = values
            .iter()
            .copied()
            .cycle()
            .take(width * self.instances as usize)
            .collect::<Vec<_>>();
        if self.backend == NativeBackend::Scalar {
            return expanded;
        }
        let mut packed = vec![0.0; expanded.len()];
        for group in 0..self.instances as usize / 4 {
            for component in 0..width {
                for lane in 0..4 {
                    packed[(group * width + component) * 4 + lane] =
                        expanded[(group * 4 + lane) * width + component];
                }
            }
        }
        packed
    }
}

pub(super) struct NativeSession {
    metadata: Arc<Metadata>,
    library: Arc<LoadedLibrary>,
    inputs: BTreeMap<u32, Vec<f32>>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    packed_state: BTreeMap<u32, Vec<f32>>,
    candidate: BTreeMap<u32, Vec<f32>>,
    attempted_turns: u64,
}
impl NativeSession {
    pub(super) fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        let replacements = updates
            .iter()
            .map(|(name, values)| {
                let input = self
                    .metadata
                    .inputs
                    .iter()
                    .find(|input| &input.name == name)
                    .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
                let batch = input.width * self.metadata.instances as usize;
                if values.len() != input.width && values.len() != batch {
                    return Err(BatchedExecutionError::InputLength {
                        name: name.clone(),
                        expected_single: input.width,
                        expected_batch: batch,
                        actual: values.len(),
                    });
                }
                Ok((input.slot, self.metadata.pack(values, input.width)))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        self.inputs.extend(replacements);
        Ok(())
    }
    pub(super) fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            self.attempted_turns = self.attempted_turns.saturating_add(1);
            let inputs = self
                .metadata
                .inputs
                .iter()
                .map(|input| self.inputs[&input.slot].as_ptr())
                .collect::<Vec<_>>();
            let states = self
                .metadata
                .states
                .iter()
                .map(|state| self.packed_state[&state.slot].as_ptr())
                .collect::<Vec<_>>();
            let candidates = self
                .metadata
                .states
                .iter()
                .map(|state| {
                    self.candidate
                        .get_mut(&state.slot)
                        .expect("validated state")
                        .as_mut_ptr()
                })
                .collect::<Vec<_>>();
            let extent = self.metadata.instances as usize
                / if self.metadata.backend == NativeBackend::Simd {
                    4
                } else {
                    1
                };
            // SAFETY: trusted bundle's validated layouts allocate each pointer table
            // in its saved ABI order; all buffers remain live and disjoint here.
            let fault = unsafe {
                (self.library.turn)(
                    inputs.as_ptr(),
                    states.as_ptr(),
                    candidates.as_ptr(),
                    extent,
                )
            };
            let code = (fault & 0xff) as usize;
            if code != 0 {
                let constraint = self.metadata.constraints.get(code - 1).ok_or_else(|| {
                    BatchedExecutionError::Native(
                        "library returned an invalid integrity code".into(),
                    )
                })?;
                return Err(BatchedExecutionError::Integrity(BatchedIntegrityFault {
                    attempted_turn: self.attempted_turns,
                    instance: (fault >> 8) as u32,
                    constraint: IntegrityConstraintId::new(constraint.id),
                    constraint_name: constraint.name.clone().into_boxed_str(),
                }));
            }
            std::mem::swap(&mut self.packed_state, &mut self.candidate);
            for descriptor in &self.metadata.states {
                let packed = &self.packed_state[&descriptor.slot];
                let published = self
                    .state
                    .get_mut(&CellSlotId::new(descriptor.slot))
                    .expect("validated state");
                if self.metadata.backend == NativeBackend::Scalar {
                    published.copy_from_slice(packed);
                } else {
                    for group in 0..self.metadata.instances as usize / 4 {
                        for component in 0..descriptor.width {
                            for lane in 0..4 {
                                published[(group * 4 + lane) * descriptor.width + component] =
                                    packed[(group * descriptor.width + component) * 4 + lane];
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn state(&self) -> &BTreeMap<CellSlotId, Vec<f32>> {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> Metadata {
        Metadata {
            target: Target::current(),
            backend: NativeBackend::Scalar,
            library_file: library_file().into(),
            library_sha256: "unused-in-layout-test".into(),
            instances: 4,
            inputs: vec![Input {
                name: "input".into(),
                slot: 1,
                width: 1,
            }],
            states: vec![State {
                slot: 2,
                width: 1,
                initializer_bits: vec![0],
            }],
            constraints: vec![],
            exports: BTreeMap::from([("state".into(), 2)]),
            initial_inputs: BTreeMap::from([("input".into(), vec![1f32.to_bits()])]),
        }
    }

    #[test]
    fn metadata_validation_rejects_target_mismatch_even_with_valid_digest() {
        let mut value = metadata();
        value.target.arch = "incompatible-architecture".into();
        let bytes = serde_json::to_vec(&value).unwrap();
        let manifest = Manifest {
            format_version: FORMAT_VERSION,
            metadata_sha256: digest(&bytes),
            metadata: value,
        };
        let restored: Manifest =
            serde_json::from_slice(&serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert_eq!(
            restored.metadata_sha256,
            digest(&serde_json::to_vec(&restored.metadata).unwrap())
        );
        assert!(
            matches!(restored.metadata.validate(), Err(Error::Bundle(message)) if message.contains("target"))
        );
    }

    #[test]
    fn metadata_validation_checks_native_layout_before_allocation() {
        metadata().validate().unwrap();
        let mut value = metadata();
        value.states[0].width = usize::MAX;
        assert!(value.validate().is_err());
        let mut value = metadata();
        value.states[0].slot = value.inputs[0].slot;
        assert!(value.validate().is_err());
        let mut value = metadata();
        value.exports.insert("invalid".into(), 999);
        assert!(value.validate().is_err());
        let mut value = metadata();
        value.backend = NativeBackend::Simd;
        value.instances = 3;
        assert!(value.validate().is_err());
        let mut value = metadata();
        value.library_file = "../outside.dylib".into();
        assert!(value.validate().is_err());
    }
}
