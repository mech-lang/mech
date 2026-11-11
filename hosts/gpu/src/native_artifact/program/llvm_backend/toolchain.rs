//! LLVM is a compile-time dependency only. No process or toolchain state is
//! retained by the emitted program or consulted during native execution.
use super::machine::MachineFacts;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

type Result<T> = std::result::Result<T, String>;
static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct NativeLlvmToolchain {
    opt: PathBuf,
    llc: PathBuf,
    linker: PathBuf,
    diagnostics: Option<PathBuf>,
}
impl NativeLlvmToolchain {
    /// Select an installed LLVM bin directory explicitly. Tools run only while
    /// compiling; the compiler and this toolchain remain trusted build inputs.
    pub fn new(bin_directory: impl AsRef<Path>) -> Self {
        Self {
            opt: bin_directory.as_ref().join("opt"),
            llc: bin_directory.as_ref().join("llc"),
            linker: PathBuf::from("/usr/bin/ld"),
            diagnostics: None,
        }
    }
    /// Retain unique, compiler-owned work directories for build inspection.
    pub fn with_diagnostics(mut self, root: impl AsRef<Path>) -> Self {
        self.diagnostics = Some(root.as_ref().to_owned());
        self
    }
    fn fingerprint(&self, linker: &Path) -> Result<[u8; 32]> {
        let mut files = BTreeSet::new();
        for p in [self.opt.as_path(), self.llc.as_path(), linker] {
            files.insert(
                fs::canonicalize(p).map_err(|e| format!("LLVM tool {}: {e}", p.display()))?,
            );
            // Record the actual LLVM implementation shared libraries as well
            // as launchers. The emitted machine Object has its own digest.
            if p != linker {
                let dir = p
                    .parent()
                    .ok_or("LLVM tool has no directory")?
                    .join("../lib");
                if dir.is_dir() {
                    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
                        let p = entry.map_err(|e| e.to_string())?.path();
                        let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if n.starts_with("libLLVM") && n.ends_with(".dylib") {
                            files.insert(fs::canonicalize(p).map_err(|e| e.to_string())?);
                        }
                    }
                }
            }
        }
        let mut digest = Sha256::new();
        digest.update(b"mech.llvm-toolchain-files.v1\0");
        for p in files {
            let name = p.to_string_lossy();
            digest.update((name.len() as u64).to_le_bytes());
            digest.update(name.as_bytes());
            let mut file = fs::File::open(&p).map_err(|e| e.to_string())?;
            let mut h = Sha256::new();
            let mut buf = [0u8; 65536];
            loop {
                let n = file.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            digest.update(h.finalize());
        }
        Ok(digest.finalize().into())
    }
    pub(in crate::native_artifact::program) fn compile(
        &self,
        ir: &str,
        names: &[String],
    ) -> Result<LlvmCode> {
        if !cfg!(all(target_arch = "aarch64", target_os = "macos")) {
            return Err("LLVM native image requires Apple AArch64".into());
        }
        let linker = if self.linker == Path::new("/usr/bin/ld") {
            let output = Command::new("/usr/bin/xcrun")
                .args(["--find", "ld"])
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err("cannot resolve the selected native linker".into());
            }
            PathBuf::from(
                std::str::from_utf8(&output.stdout)
                    .map_err(|e| e.to_string())?
                    .trim(),
            )
        } else {
            self.linker.clone()
        };
        let fingerprint = self.fingerprint(&linker)?;
        let work = Work::new(self.diagnostics.as_deref())?;
        fs::write(work.path.join("native.ll"), ir).map_err(|e| e.to_string())?;
        run(
            Command::new(&self.opt).current_dir(&work.path).args([
                "-passes=default<O3>",
                "-mcpu=apple-m3",
                "-S",
                "native.ll",
                "-o",
                "optimized.ll",
            ]),
            "LLVM IR optimization",
            &work.path,
        )?;
        run(
            Command::new(&self.llc).current_dir(&work.path).args([
                "-O=3",
                "-global-isel=0",
                "-enable-machine-outliner=never",
                "-mcpu=apple-m3",
                "-relocation-model=pic",
                "-filetype=obj",
                "-pass-remarks-analysis=prologepilog",
                "-pass-remarks-filter=prolog-epilog",
                "-pass-remarks-output=remarks.yaml",
                "optimized.ll",
                "-o",
                "emitted.o",
            ]),
            "LLVM machine emission",
            &work.path,
        )?;
        // Let the native linker finish literal-pool coalescing before deriving
        // any admitted size or Object identity. The metadata lives in its own
        // section, so subsequent assembly must preserve this completed code.
        run(
            Command::new(&linker).current_dir(&work.path).args([
                "-r",
                "-arch",
                "arm64",
                "-o",
                "native.o",
                "emitted.o",
            ]),
            "LLVM machine canonicalization",
            &work.path,
        )?;
        let object = fs::read(work.path.join("native.o")).map_err(|e| e.to_string())?;
        let remarks =
            fs::read_to_string(work.path.join("remarks.yaml")).map_err(|e| e.to_string())?;
        let facts = MachineFacts::read(&object, &remarks, names)?;
        fs::write(
            work.path.join("machine-facts.json"),
            serde_json::to_vec_pretty(&facts).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if self.fingerprint(&linker)? != fingerprint {
            return Err("LLVM toolchain changed during machine compilation".into());
        }
        Ok(LlvmCode {
            object,
            facts,
            toolchain: fingerprint,
            work,
            linker,
        })
    }
}

pub(in crate::native_artifact::program) struct LlvmCode {
    pub object: Vec<u8>,
    pub facts: MachineFacts,
    pub toolchain: [u8; 32],
    work: Work,
    linker: PathBuf,
}
impl LlvmCode {
    pub(in crate::native_artifact::program) fn assemble(&self, metadata: &[u8]) -> Result<Vec<u8>> {
        fs::write(self.work.path.join("metadata.o"), metadata).map_err(|e| e.to_string())?;
        run(
            Command::new(&self.linker)
                .current_dir(&self.work.path)
                .args([
                    "-r",
                    "-arch",
                    "arm64",
                    "-o",
                    "program.o",
                    "native.o",
                    "metadata.o",
                ]),
            "LLVM same-Object assembly",
            &self.work.path,
        )?;
        let bytes = fs::read(self.work.path.join("program.o")).map_err(|e| e.to_string())?;
        self.facts.audit_assembled(&self.object, &bytes)?;
        Ok(bytes)
    }
}

fn run(command: &mut Command, phase: &str, path: &Path) -> Result<()> {
    let output = command.output().map_err(|e| format!("{phase}: {e}"))?;
    let stem = phase.replace([' ', '/'], "-");
    fs::write(path.join(format!("{stem}.stdout")), &output.stdout).map_err(|e| e.to_string())?;
    fs::write(path.join(format!("{stem}.stderr")), &output.stderr).map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "{phase}: {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

struct Work {
    path: PathBuf,
    retain: bool,
}
impl Work {
    fn new(root: Option<&Path>) -> Result<Self> {
        let retain = root.is_some();
        let root = root.map_or_else(std::env::temp_dir, Path::to_owned);
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        for _ in 0..100 {
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            let path = root.join(format!(
                "mech-llvm-{}-{time}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::DirBuilder::new().create(&path) {
                Ok(()) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                            .map_err(|e| e.to_string())?;
                    }
                    return Ok(Self { path, retain });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("cannot create private LLVM work directory".into())
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        if !self.retain {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
