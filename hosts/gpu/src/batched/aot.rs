use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs, mem,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use cranelift_codegen::settings::{self, Configurable};
use cranelift_module::{Linkage, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use mech_core::CellSlotId;
use sha2::{Digest, Sha256};

use super::{
    BatchedExecutionError, BatchedFaultRecorder, BatchedIntegrityFault, FixedShapeKernel,
    jit::{NativeMathSymbols, NativeTurn, define_native_turn},
};

const AOT_ENTRY_POINT: &[u8] = b"mech_fixed_numeric_turn\0";

struct AotKernel {
    _library: libloading::Library,
    turn: NativeTurn,
    path: PathBuf,
}

/// A reusable native library emitted from one fixed-shape Mech compute region.
#[derive(Clone)]
pub struct BatchedAotCpuArtifact {
    program: Arc<FixedShapeKernel>,
    kernel: Arc<AotKernel>,
}

/// A resident CPU session backed by a saved Cranelift native library.
pub struct BatchedAotCpuSession {
    program: Arc<FixedShapeKernel>,
    kernel: Arc<AotKernel>,
    inputs: BTreeMap<CellSlotId, Vec<f32>>,
    state: BTreeMap<CellSlotId, Vec<f32>>,
    next_state: BTreeMap<CellSlotId, Vec<f32>>,
    input_pointers: Vec<*const f32>,
    state_pointers: Vec<*const f32>,
    next_state_pointers: Vec<*mut f32>,
    faults: BatchedFaultRecorder,
}

impl FixedShapeKernel {
    /// Emits or reuses a native dynamic library in the default AOT cache.
    ///
    /// Set `MECH_AOT_CACHE_DIR` to choose a durable cache location. Otherwise
    /// the host temporary directory is used.
    pub fn compile_aot_cpu(&self) -> Result<BatchedAotCpuArtifact, BatchedExecutionError> {
        self.compile_aot_cpu_to(default_cache_dir())
    }

    /// Emits or reuses a native dynamic library under `directory`.
    pub fn compile_aot_cpu_to(
        &self,
        directory: impl AsRef<Path>,
    ) -> Result<BatchedAotCpuArtifact, BatchedExecutionError> {
        let path = emit_aot_library(self, directory.as_ref())?;
        let kernel = load_aot_library(path)?;
        Ok(BatchedAotCpuArtifact {
            program: Arc::new(self.clone()),
            kernel: Arc::new(kernel),
        })
    }

    /// Creates a resident session from an emitted or cached AOT library.
    pub fn prepare_aot_cpu(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedAotCpuSession, BatchedExecutionError> {
        self.compile_aot_cpu()?.prepare(inputs)
    }
}

impl BatchedAotCpuArtifact {
    /// Returns the saved native library loaded by this artifact.
    pub fn path(&self) -> &Path {
        &self.kernel.path
    }

    /// Creates a fresh stateful session without recompiling the native code.
    pub fn prepare(
        &self,
        inputs: &BTreeMap<String, Vec<f32>>,
    ) -> Result<BatchedAotCpuSession, BatchedExecutionError> {
        let inputs = self.program.expand_inputs(inputs)?;
        let state = self.program.initial_state();
        let next_state = state
            .iter()
            .map(|(slot, values)| (*slot, vec![0.0; values.len()]))
            .collect();
        let input_pointers = self
            .program
            .inputs
            .iter()
            .map(|input| inputs[&input.slot].as_ptr())
            .collect();
        let mut session = BatchedAotCpuSession {
            program: Arc::clone(&self.program),
            kernel: Arc::clone(&self.kernel),
            inputs,
            state,
            next_state,
            input_pointers,
            state_pointers: Vec::with_capacity(self.program.states.len()),
            next_state_pointers: Vec::with_capacity(self.program.states.len()),
            faults: BatchedFaultRecorder::default(),
        };
        session.refresh_state_pointers();
        Ok(session)
    }
}

impl BatchedAotCpuSession {
    pub fn artifact_path(&self) -> &Path {
        &self.kernel.path
    }

    pub fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        for (name, values) in updates {
            let input = self
                .program
                .inputs
                .iter()
                .find(|input| input.name == *name)
                .ok_or_else(|| BatchedExecutionError::MissingInput(name.clone()))?;
            self.inputs
                .insert(input.slot, self.program.expand_input(input, values)?);
        }
        self.input_pointers = self
            .program
            .inputs
            .iter()
            .map(|input| self.inputs[&input.slot].as_ptr())
            .collect();
        Ok(())
    }

    pub fn dispatch_turns(&mut self, turns: u32) -> Result<(), BatchedExecutionError> {
        if turns == 0 {
            return Err(BatchedExecutionError::ZeroTurns);
        }
        for _ in 0..turns {
            let attempted_turn = self.faults.next_turn();
            self.refresh_state_pointers();
            // SAFETY: The library exports the same four-argument ABI used by
            // the JIT path. Pointer tables and backing buffers remain live for
            // the call and the generated loop is bounded by `instances`.
            let packed_fault = unsafe {
                (self.kernel.turn)(
                    self.input_pointers.as_ptr(),
                    self.state_pointers.as_ptr(),
                    self.next_state_pointers.as_ptr(),
                    self.program.instances as usize,
                )
            };
            if let Some(fault) = self
                .program
                .failed_packed_constraint(packed_fault, attempted_turn)
            {
                return Err(self.faults.record(fault));
            }
            mem::swap(&mut self.state, &mut self.next_state);
        }
        Ok(())
    }

    pub fn state(&self) -> &BTreeMap<CellSlotId, Vec<f32>> {
        &self.state
    }

    pub const fn fault_count(&self) -> u64 {
        self.faults.fault_count
    }

    pub const fn attempted_turns(&self) -> u64 {
        self.faults.attempted_turns()
    }

    pub fn last_fault(&self) -> Option<&BatchedIntegrityFault> {
        self.faults.last_fault.as_ref()
    }

    fn refresh_state_pointers(&mut self) {
        self.state_pointers.clear();
        self.next_state_pointers.clear();
        for state in &self.program.states {
            self.state_pointers.push(self.state[&state.slot].as_ptr());
            self.next_state_pointers
                .push(self.next_state.get_mut(&state.slot).unwrap().as_mut_ptr());
        }
    }
}

fn default_cache_dir() -> PathBuf {
    std::env::var_os("MECH_AOT_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("mech-aot-v1"))
}

fn emit_aot_library(
    program: &FixedShapeKernel,
    directory: &Path,
) -> Result<PathBuf, BatchedExecutionError> {
    fs::create_dir_all(directory).map_err(aot_error)?;
    let key = artifact_key(program);
    let library_path = directory.join(format!("mech-{key}.{}", dynamic_library_extension()));
    if library_path.is_file() {
        return Ok(library_path);
    }

    let object_path = directory.join(format!("mech-{key}.{}", object_extension()));
    let mut flag_builder = settings::builder();
    flag_builder.set("opt_level", "speed").map_err(aot_error)?;
    flag_builder.set("is_pic", "true").map_err(aot_error)?;
    let isa_builder = cranelift_native::builder().map_err(aot_error)?;
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(aot_error)?;
    let builder = ObjectBuilder::new(isa, "mech_fixed_shape_aot", default_libcall_names())
        .map_err(aot_error)?;
    let mut module = ObjectModule::new(builder);
    define_native_turn(
        &mut module,
        program,
        Linkage::Export,
        NativeMathSymbols {
            sin: "sinf",
            cos: "cosf",
            sqrt: "sqrtf",
            ceil: "ceilf",
            atan2: "atan2f",
        },
    )?;
    let bytes = module.finish().emit().map_err(aot_error)?;
    fs::write(&object_path, bytes).map_err(aot_error)?;
    link_dynamic_library(&object_path, &library_path)?;
    Ok(library_path)
}

fn load_aot_library(path: PathBuf) -> Result<AotKernel, BatchedExecutionError> {
    // SAFETY: The path was produced by `emit_aot_library` for this process's
    // host target. The library remains owned by `AotKernel` for the lifetime
    // of the copied function pointer.
    let library = unsafe { libloading::Library::new(&path) }.map_err(aot_error)?;
    let turn = unsafe {
        *library
            .get::<NativeTurn>(AOT_ENTRY_POINT)
            .map_err(aot_error)?
    };
    Ok(AotKernel {
        _library: library,
        turn,
        path,
    })
}

fn artifact_key(program: &FixedShapeKernel) -> String {
    let mut hasher = Sha256::new();
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(b"cranelift-0.131.3");
    hasher.update(std::env::consts::ARCH.as_bytes());
    hasher.update(std::env::consts::OS.as_bytes());
    hasher.update(format!("{program:#?}").as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn link_dynamic_library(
    object_path: &Path,
    library_path: &Path,
) -> Result<(), BatchedExecutionError> {
    let compiler = std::env::var_os("CC").unwrap_or_else(|| OsString::from("cc"));
    let temporary = library_path.with_extension(format!(
        "{}.{}.tmp",
        dynamic_library_extension(),
        std::process::id()
    ));
    let mut command = Command::new(&compiler);
    if cfg!(target_os = "macos") {
        command.arg("-dynamiclib");
    } else if cfg!(target_family = "unix") {
        command.arg("-shared");
    } else {
        return Err(aot_error(
            "the AOT linker currently supports macOS and Unix hosts",
        ));
    }
    let output = command
        .arg(object_path)
        .arg("-o")
        .arg(&temporary)
        .arg("-lm")
        .output()
        .map_err(aot_error)?;
    if !output.status.success() {
        return Err(aot_error(format!(
            "{} failed while linking {}: {}",
            Path::new(&compiler).display(),
            library_path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    match fs::rename(&temporary, library_path) {
        Ok(()) => Ok(()),
        Err(_) if library_path.is_file() => {
            let _ = fs::remove_file(&temporary);
            Ok(())
        }
        Err(error) => Err(aot_error(error)),
    }
}

const fn dynamic_library_extension() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

const fn object_extension() -> &'static str {
    if cfg!(target_os = "windows") {
        "obj"
    } else {
        "o"
    }
}

fn aot_error(error: impl std::fmt::Display) -> BatchedExecutionError {
    BatchedExecutionError::Native(format!("Cranelift AOT: {error}"))
}
