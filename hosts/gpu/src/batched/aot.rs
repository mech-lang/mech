use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs, io,
    io::Write,
    mem,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use cranelift_codegen::isa::{OwnedTargetIsa, TargetIsa};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_module::{Linkage, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use mech_core::CellSlotId;
use sha2::{Digest, Sha256};

use super::{
    BatchedExecutionError, BatchedFaultRecorder, BatchedIntegrityFault, FixedShapeKernel,
    jit::{NativeMathSymbols, NativeTurn, define_native_turn, stage_input_updates},
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
    /// a private per-user directory under the host temporary directory is used.
    pub fn compile_aot_cpu(&self) -> Result<BatchedAotCpuArtifact, BatchedExecutionError> {
        self.compile_aot_cpu_to(default_cache_dir()?)
    }

    /// Emits or reuses a native dynamic library under `directory`.
    ///
    /// On Unix the cache must be owned by this user, private (mode 0700),
    /// and reached through directories that other users cannot replace.
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

    /// Validates every update before replacing any backing buffer.
    pub fn update_inputs(
        &mut self,
        updates: &BTreeMap<String, Vec<f32>>,
    ) -> Result<(), BatchedExecutionError> {
        let staged = stage_input_updates(&self.program, updates)?;
        for (slot, values) in staged {
            self.inputs.insert(slot, values);
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

fn default_cache_dir() -> Result<PathBuf, BatchedExecutionError> {
    if let Some(path) = std::env::var_os("MECH_AOT_CACHE_DIR") {
        return Ok(PathBuf::from(path));
    }
    #[cfg(unix)]
    return Ok(std::env::temp_dir().join(format!("mech-aot-v2-{}", effective_user_id())));
    #[cfg(not(unix))]
    Err(aot_error(
        "the AOT linker currently supports macOS and Unix hosts",
    ))
}

fn emit_aot_library(
    program: &FixedShapeKernel,
    directory: &Path,
) -> Result<PathBuf, BatchedExecutionError> {
    let directory = private_cache_directory(directory)?;
    // Fingerprint and emission share this exact resolved native target.
    let isa = native_object_isa()?;
    let key = artifact_key(program, isa.as_ref());
    let library_path = directory.join(format!("mech-{key}.{}", dynamic_library_extension()));
    if cached_library_exists(&library_path)? {
        return Ok(library_path);
    }

    // Each invocation owns both linker inputs and outputs. Concurrent callers
    // can only observe an already complete artifact at the final cache path.
    let work = AotWorkDirectory::create(&directory)?;
    let object_path = work.path.join(format!("module.{}", object_extension()));
    let linked_path = work
        .path
        .join(format!("module.{}", dynamic_library_extension()));
    let bytes = emit_native_object_with_isa("mech_fixed_shape_aot", isa, |module| {
        define_native_turn(
            module,
            program,
            Linkage::Export,
            NativeMathSymbols {
                sin: "sinf",
                cos: "cosf",
                sqrt: "sqrtf",
                ceil: "ceilf",
                fmod: "fmodf",
                atan2: "atan2f",
            },
        )?;
        Ok(())
    })?;
    let mut object = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&object_path)
        .map_err(aot_error)?;
    object.write_all(&bytes).map_err(aot_error)?;
    drop(object);
    link_dynamic_library(&object_path, &linked_path)?;
    validate_cached_library(&linked_path)?;
    // Fail closed for an unexpected file/symlink installed during compilation.
    // Replacing another complete, qualified artifact is safe for readers: the
    // rename never exposes linker output while it is still being written.
    cached_library_exists(&library_path)?;
    fs::rename(&linked_path, &library_path).map_err(aot_error)?;
    Ok(library_path)
}

/// Shared IROS AOT emission used by scalar f32 and autonomous f64 lowering.
pub(crate) fn emit_native_object(
    name: &str,
    define: impl FnOnce(&mut ObjectModule) -> Result<(), BatchedExecutionError>,
) -> Result<Vec<u8>, BatchedExecutionError> {
    emit_native_object_with_isa(name, native_object_isa()?, define)
}

fn native_object_isa() -> Result<OwnedTargetIsa, BatchedExecutionError> {
    let mut flags = settings::builder();
    flags.set("opt_level", "speed").map_err(aot_error)?;
    flags.set("is_pic", "true").map_err(aot_error)?;
    cranelift_native::builder()
        .map_err(aot_error)?
        .finish(settings::Flags::new(flags))
        .map_err(aot_error)
}

fn emit_native_object_with_isa(
    name: &str,
    isa: OwnedTargetIsa,
    define: impl FnOnce(&mut ObjectModule) -> Result<(), BatchedExecutionError>,
) -> Result<Vec<u8>, BatchedExecutionError> {
    let mut module = ObjectModule::new(
        ObjectBuilder::new(isa, name, default_libcall_names()).map_err(aot_error)?,
    );
    define(&mut module)?;
    module.finish().emit().map_err(aot_error)
}

fn load_aot_library(path: PathBuf) -> Result<AotKernel, BatchedExecutionError> {
    validate_cached_library(&path)?;
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

fn artifact_key(program: &FixedShapeKernel, isa: &dyn TargetIsa) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"mech-fixed-shape-aot-v2\0four-pointer-native-turn-f32-u64\0");
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(b"cranelift-0.131.3");
    hasher.update(std::env::consts::ARCH.as_bytes());
    hasher.update(std::env::consts::OS.as_bytes());
    hasher.update(target_fingerprint(isa).as_bytes());
    hasher.update(format!("{program:#?}").as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn target_fingerprint(isa: &dyn TargetIsa) -> String {
    let mut flags = isa
        .isa_flags()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    flags.sort_unstable();
    format!(
        "triple={}\ncall_conv={:?}\npointer_type={}\nendianness={:?}\nshared_flags={}\nisa_flags={flags:?}\n",
        isa.triple(),
        isa.default_call_conv(),
        isa.pointer_type(),
        isa.endianness(),
        isa.flags(),
    )
}

fn link_dynamic_library(
    object_path: &Path,
    library_path: &Path,
) -> Result<(), BatchedExecutionError> {
    let compiler = std::env::var_os("CC").unwrap_or_else(|| OsString::from("cc"));
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
        .arg(library_path)
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
    Ok(())
}

#[cfg(unix)]
fn effective_user_id() -> u32 {
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    // SAFETY: geteuid has no arguments and returns the process effective uid.
    unsafe { geteuid() }
}

fn private_cache_directory(directory: &Path) -> Result<PathBuf, BatchedExecutionError> {
    #[cfg(not(unix))]
    return Err(aot_error(
        "the AOT linker currently supports macOS and Unix hosts",
    ));
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        match fs::symlink_metadata(directory) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(directory)
                    .map_err(aot_error)?;
            }
            Err(error) => return Err(aot_error(error)),
            Ok(metadata) if !metadata.is_dir() => {
                return Err(aot_error("AOT cache must be a directory, not a symlink"));
            }
            Ok(_) => {}
        }
        let metadata = fs::symlink_metadata(directory).map_err(aot_error)?;
        let uid = effective_user_id();
        if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
            return Err(aot_error(
                "AOT cache must be a private directory owned by the current user (mode 0700)",
            ));
        }
        // Canonicalize platform aliases such as macOS /tmp, then reject any
        // ancestor under another user's control. Root-owned sticky /tmp is
        // safe: another uid cannot rename this user's child directory there.
        let canonical = fs::canonicalize(directory).map_err(aot_error)?;
        validate_resolved_cache_directory(&canonical, &metadata)?;
        Ok(canonical)
    }
}

#[cfg(unix)]
fn validate_resolved_cache_directory(
    canonical: &Path,
    original: &fs::Metadata,
) -> Result<(), BatchedExecutionError> {
    use std::os::unix::fs::MetadataExt;
    let uid = effective_user_id();
    let check_leaf = || {
        let current = fs::symlink_metadata(canonical).map_err(aot_error)?;
        if !current.is_dir()
            || current.uid() != uid
            || current.mode() & 0o077 != 0
            || (current.dev(), current.ino()) != (original.dev(), original.ino())
        {
            return Err(aot_error(
                "AOT cache directory identity or private ownership changed during path resolution",
            ));
        }
        Ok(())
    };
    check_leaf()?;
    // Check from the trusted root toward the leaf. Once each parent is
    // trusted, another uid cannot replace the next current-user/root-owned
    // component. Recheck the leaf after establishing the whole chain.
    let mut ancestors = canonical.ancestors().skip(1).collect::<Vec<_>>();
    ancestors.reverse();
    for parent in ancestors {
        let metadata = fs::symlink_metadata(parent).map_err(aot_error)?;
        if !metadata.is_dir()
            || (metadata.uid() != uid && metadata.uid() != 0)
            || (metadata.mode() & 0o022 != 0 && metadata.mode() & 0o1000 == 0)
        {
            return Err(aot_error(format!(
                "AOT cache has an untrusted writable parent: {}",
                parent.display()
            )));
        }
    }
    check_leaf()
}

fn cached_library_exists(path: &Path) -> Result<bool, BatchedExecutionError> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            validate_cached_library(path)?;
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(aot_error(error)),
    }
}

fn validate_cached_library(path: &Path) -> Result<(), BatchedExecutionError> {
    let metadata = fs::symlink_metadata(path).map_err(aot_error)?;
    if !metadata.is_file() {
        return Err(aot_error(
            "AOT cache artifact must be a regular file, not a symlink",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != effective_user_id() || metadata.mode() & 0o022 != 0 {
            return Err(aot_error(
                "AOT cache artifact has an untrusted owner or writable permissions",
            ));
        }
    }
    Ok(())
}

static NEXT_AOT_WORK_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct AotWorkDirectory {
    path: PathBuf,
}

impl AotWorkDirectory {
    fn create(parent: &Path) -> Result<Self, BatchedExecutionError> {
        for _ in 0..128 {
            let sequence = NEXT_AOT_WORK_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(".build-{}-{sequence}", std::process::id()));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(aot_error(error)),
            }
        }
        Err(aot_error(
            "could not create a unique private AOT build directory",
        ))
    }
}

impl Drop for AotWorkDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
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

#[cfg(all(test, unix))]
mod tests {
    use super::super::jit::tests::{initial_inputs, input_update_program, rejected_updates};
    use super::*;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

    #[test]
    fn aot_rejected_updates_preserve_backing_buffers_before_dispatch() {
        let program = input_update_program();
        let inputs = initial_inputs();
        let directory = AotWorkDirectory::create(&std::env::temp_dir()).unwrap();
        let artifact = program.compile_aot_cpu_to(&directory.path).unwrap();
        let mut session = artifact.prepare(&inputs).unwrap();
        let initial = session.inputs.clone();
        let pointers = session.input_pointers.clone();
        let addresses = session
            .inputs
            .iter()
            .map(|(k, v)| (*k, v.as_ptr()))
            .collect::<Vec<_>>();
        for updates in rejected_updates() {
            assert!(session.update_inputs(&updates).is_err());
            // Check ownership before executing code that consumes raw pointers.
            assert_eq!(session.inputs, initial);
            assert_eq!(session.input_pointers, pointers);
            assert_eq!(
                session
                    .inputs
                    .iter()
                    .map(|(k, v)| (*k, v.as_ptr()))
                    .collect::<Vec<_>>(),
                addresses
            );
        }
        let mut scalar = program.prepare_cpu(&inputs).unwrap();
        session.dispatch_turns(3).unwrap();
        scalar.dispatch_turns(3).unwrap();
        assert_eq!(session.state(), scalar.state());
        let updates = BTreeMap::from([
            ("input".to_owned(), vec![2.0]),
            ("scale".to_owned(), vec![3.0]),
        ]);
        session.update_inputs(&updates).unwrap();
        scalar.update_inputs(&updates).unwrap();
        for (index, input) in program.inputs.iter().enumerate() {
            assert_eq!(
                session.input_pointers[index],
                session.inputs[&input.slot].as_ptr()
            );
        }
        session.dispatch_turns(2).unwrap();
        scalar.dispatch_turns(2).unwrap();
        assert_eq!(session.state(), scalar.state());
    }

    #[test]
    fn cache_rejects_nonprivate_directories_symlinks_and_writable_artifacts() {
        let root = AotWorkDirectory::create(&std::env::temp_dir()).unwrap();
        let cache = root.path.join("cache");
        let canonical = private_cache_directory(&cache).unwrap();
        let metadata = fs::symlink_metadata(&canonical).unwrap();
        assert_eq!(metadata.uid(), effective_user_id());
        assert_eq!(metadata.mode() & 0o777, 0o700);
        assert_eq!(private_cache_directory(&cache).unwrap(), canonical);

        fs::set_permissions(&cache, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(private_cache_directory(&cache).is_err());
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o700)).unwrap();
        let redirected = root.path.join("redirected");
        symlink(&cache, &redirected).unwrap();
        assert!(private_cache_directory(&redirected).is_err());

        let library = cache.join("module.dylib");
        let outside = root.path.join("outside");
        fs::write(&outside, b"must never load through a cache symlink").unwrap();
        symlink(&outside, &library).unwrap();
        assert!(cached_library_exists(&library).is_err());
        fs::remove_file(&library).unwrap();
        fs::write(&library, b"test ownership only; never loaded").unwrap();
        fs::set_permissions(&library, fs::Permissions::from_mode(0o666)).unwrap();
        assert!(cached_library_exists(&library).is_err());
        fs::set_permissions(&library, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(cached_library_exists(&library).unwrap());

        let writable_parent = root.path.join("shared");
        fs::create_dir(&writable_parent).unwrap();
        fs::set_permissions(&writable_parent, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(private_cache_directory(&writable_parent.join("private-child")).is_err());
    }

    #[test]
    fn resolved_cache_must_be_the_same_private_directory_checked_before_resolution() {
        let root = AotWorkDirectory::create(&std::env::temp_dir()).unwrap();
        let original = private_cache_directory(&root.path.join("original")).unwrap();
        let replacement = private_cache_directory(&root.path.join("replacement")).unwrap();
        let checked = fs::symlink_metadata(&original).unwrap();
        validate_resolved_cache_directory(&original, &checked).unwrap();
        // Deterministically model canonicalize resolving to a replaced leaf.
        // Even a same-user 0700 replacement must fail the device/inode binding;
        // an attacker-owned replacement is therefore never silently accepted.
        assert!(validate_resolved_cache_directory(&replacement, &checked).is_err());
        fs::remove_dir(&original).unwrap();
        symlink(&replacement, &original).unwrap();
        assert!(validate_resolved_cache_directory(&original, &checked).is_err());
    }

    #[test]
    fn isa_features_shared_flags_and_abi_change_cache_identity() {
        let program = input_update_program();
        let native = native_object_isa().unwrap();
        let native_key = artifact_key(&program, native.as_ref());
        assert_eq!(
            native_key,
            artifact_key(&program, native_object_isa().unwrap().as_ref())
        );

        let mut changed_flags = settings::builder();
        changed_flags.set("opt_level", "none").unwrap();
        changed_flags.set("is_pic", "true").unwrap();
        let changed = cranelift_native::builder()
            .unwrap()
            .finish(settings::Flags::new(changed_flags))
            .unwrap();
        assert_ne!(native_key, artifact_key(&program, changed.as_ref()));

        // Toggle an available native feature without emitting or executing its
        // instructions; the actual resolved flags must affect cache identity.
        let feature = if cfg!(target_arch = "aarch64") {
            "has_lse"
        } else {
            "has_avx"
        };
        if cfg!(any(target_arch = "aarch64", target_arch = "x86_64")) {
            let mut keys = Vec::new();
            for value in ["false", "true"] {
                let mut builder = cranelift_native::builder().unwrap();
                builder.set(feature, value).unwrap();
                let mut flags = settings::builder();
                flags.set("opt_level", "speed").unwrap();
                flags.set("is_pic", "true").unwrap();
                let isa = builder.finish(settings::Flags::new(flags)).unwrap();
                keys.push(artifact_key(&program, isa.as_ref()));
            }
            assert_ne!(keys[0], keys[1]);
        }
        let fingerprint = target_fingerprint(native.as_ref());
        assert!(fingerprint.contains(&format!("triple={}", native.triple())));
        assert!(fingerprint.contains(&format!("call_conv={:?}", native.default_call_conv())));
        assert!(fingerprint.contains(&format!("pointer_type={}", native.pointer_type())));
    }

    #[test]
    fn concurrent_aot_builds_publish_complete_isolated_libraries() {
        let program = input_update_program();
        let root = AotWorkDirectory::create(&std::env::temp_dir()).unwrap();
        let cache = private_cache_directory(&root.path.join("cache")).unwrap();
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            let handles = (0..2)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        let artifact = program.compile_aot_cpu_to(&cache).unwrap();
                        let mut session = artifact.prepare(&initial_inputs()).unwrap();
                        session.dispatch_turns(3).unwrap();
                        (artifact.path().to_owned(), session.state().clone())
                    })
                })
                .collect::<Vec<_>>();
            let results = handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(results[0], results[1]);
        });
        let entries = fs::read_dir(&cache)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(
            entries.len(),
            1,
            "temporary objects and libraries must be cleaned up"
        );
        validate_cached_library(&entries[0]).unwrap();
    }

    #[test]
    fn simultaneous_work_directories_do_not_share_linker_paths() {
        let root = AotWorkDirectory::create(&std::env::temp_dir()).unwrap();
        let first = AotWorkDirectory::create(&root.path).unwrap();
        let second = AotWorkDirectory::create(&root.path).unwrap();
        assert_ne!(first.path, second.path);
        assert_eq!(fs::metadata(&first.path).unwrap().mode() & 0o777, 0o700);
        fs::write(first.path.join("module.o"), b"first object").unwrap();
        fs::write(second.path.join("module.o"), b"second object").unwrap();
        let first_path = first.path.clone();
        drop(first);
        assert!(!first_path.exists());
        assert_eq!(
            fs::read(second.path.join("module.o")).unwrap(),
            b"second object"
        );
    }
}
