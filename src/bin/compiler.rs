// src/main.rs
use std::{
    env,
    thread,
    io::{stdout, Read, Write, Cursor, BufRead, BufReader},
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use serde_json::Value;
use tempfile::TempDir;
use anyhow::{Context, Result};

struct Progress {
    done: usize,
    total: usize,
    finished: bool,
}

struct Spinner {
    frames: Vec<&'static str>,
    current: usize,
    last_update: Instant,
}

impl Spinner {
    fn new() -> Self {
        Self {
            frames: vec!["|", "/", "-", "\\"],
            current: 0,
            last_update: Instant::now(),
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_update) < Duration::from_millis(80) {
            return;
        }
        self.last_update = now;
        print!("\r{}", self.frames[self.current]);
        stdout().flush().ok();
        self.current = (self.current + 1) % self.frames.len();
    }

    fn clear(&self) {
        print!("\r \r");
        stdout().flush().ok();
    }
}

struct SpinnerDisplay {
    start_time: Instant,
    progress: Arc<Mutex<Progress>>,
    handle: Option<thread::JoinHandle<()>>,
}

// ------------------ CONFIG / USAGE ------------------
fn usage_and_exit() -> ! {
    eprintln!("Usage:");
    eprintln!("  mec_packer build <mec-folder> --shim-name <shim_name> --out <out_path> [--target <target>] [--cargo-flags \"...\"]");
    std::process::exit(2);
}

// ------------------ Helpers: .mec -> bytecode (placeholder) ------------------
// Replace this with a real compile step that returns actual bytecode bytes.
fn compile_mec_to_bytecode(path: &Path) -> Result<Vec<u8>> {
    println!("  [compile] reading {}", path.display());
    let mut v = Vec::new();
    File::open(path)?.read_to_end(&mut v)?;
    println!("  [compile] {} bytes", v.len());
    Ok(v)
}

// ------------------ Create ZIP from pairs (stem, bytes) ------------------
fn create_zip_from_pairs(pairs: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    println!("  [zip] building ZIP from {} entries", pairs.len());
    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buf));
        let options: FileOptions<'_, ()> =
            FileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, bytes) in pairs {
            let entry_name = format!("{}.mecb", name);
            println!("  [zip] adding {} ({} bytes)", entry_name, bytes.len());
            zip.start_file(&entry_name, options)?;
            zip.write_all(bytes)?;
        }
        zip.finish()?;
    }
    println!("  [zip] total zip size = {} bytes", buf.len());
    Ok(buf)
}

// ------------------ Write shim cargo project ------------------
fn write_shim_project(temp: &TempDir, shim_name: &str) -> Result<PathBuf> {
    let project_dir = temp.path().join(shim_name);
    fs::create_dir_all(project_dir.join("src"))?;

    // Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
zip = "5.1"
"#,
        name = shim_name
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

    // src/main.rs -- shim reads appended ZIP using the footer (last 8 bytes)
    let main_rs = r#"
use anyhow::{Result, Context};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Cursor};
use zip::read::ZipArchive;

fn run_bytecode(name: &str, bytes: &[u8]) -> Result<()> {
    // Replace this stub with your VM runner.
    println!("[shim] run '{}', {} bytes", name, bytes.len());
    println!("[shim] content (utf8 lossy): {:?}", String::from_utf8_lossy(bytes));
    Ok(())
}

fn main() -> Result<()> {
    println!("[shim] started");
    let exe_path = std::env::current_exe()?;
    println!("[shim] exe: {}", exe_path.display());
    let mut f = File::open(&exe_path)?;
    let metadata = f.metadata().context("metadata")?;
    let len = metadata.len();
    if len < 8 {
        println!("[shim] no footer present");
        return Ok(());
    }
    // read last 8 bytes => zip size
    f.seek(SeekFrom::End(-8))?;
    let mut buf = [0u8;8];
    f.read_exact(&mut buf)?;
    let zip_size = u64::from_le_bytes(buf);
    println!("[shim] detected zip size: {}", zip_size);
    if len < 8 + zip_size {
        println!("[shim] invalid sizes");
        return Ok(());
    }
    f.seek(SeekFrom::End(-(8 + zip_size as i64)))?;
    let mut zip_buf = vec![0u8; zip_size as usize];
    f.read_exact(&mut zip_buf)?;
    println!("[shim] read zip into mem ({} bytes)", zip_buf.len());
    let mut za = ZipArchive::new(Cursor::new(zip_buf))?;
    for i in 0..za.len() {
        let mut entry = za.by_index(i)?;
        let name = entry.name().to_string();
        println!("[shim] entry: {}", name);
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        run_bytecode(&name, &data)?;
    }
    println!("[shim] done");
    Ok(())
}
"#;
    fs::write(project_dir.join("src").join("main.rs"), main_rs)?;
    Ok(project_dir)
}

// ------------------ Run cargo build but suppress output; show custom progress ------------------

pub fn cargo_build_with_progress(
    project_dir: &Path,
    target: Option<&str>,
    extra_flags: Option<&str>,
) -> Result<()> {
    println!("  [cargo] building shim in: {}", project_dir.display());

    let mut cmd = Command::new("cargo");
    cmd.current_dir(project_dir)
        .arg("build")
        .arg("--release")
        .arg("--message-format=json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(t) = target {
        cmd.arg("--target").arg(t);
    }
    if let Some(flags) = extra_flags {
        for tok in flags.split_whitespace() {
            cmd.arg(tok);
        }
    }

    let mut child = cmd.spawn().context("failed to spawn cargo build")?;
    let stdout = child.stdout.take().unwrap();

    let progress = Arc::new(Mutex::new(Progress {
        done: 0,
        total: 0,
        finished: false,
    }));
    let spinner_progress = progress.clone();

    // Spinner thread
    let spinner_handle = thread::spawn(move || {
        //let spinner_chars = ['◐','◓','◑','◒'];
        let spinner_chars = ["⠁","⠂","⠄","⡀","⡈","⡐","⡠","⣀","⣁","⣂","⣄","⣌","⣔","⣤","⣥","⣦","⣮","⣶","⣷","⣿","⡿","⠿","⢟","⠟","⡛","⠛","⠫","⢋","⠋","⠍","⡉","⠉","⠑","⠡","⢁"];
        let mut idx = 0;
        while !spinner_progress.lock().unwrap().finished {
            {
                let prog = spinner_progress.lock().unwrap();
                let pct = if prog.total == 0 {
                    0.0
                } else {
                    prog.done as f64 / prog.total as f64
                };
                let total_blocks = 12;
                let filled = (pct * total_blocks as f64).round() as usize;
                let bar = format!("{}{}", "▰".repeat(filled), "▱".repeat(total_blocks - filled));
                print!(
                    "\r  [cargo] {} {}▱{:>5.1}% ({}/{}) ",
                    spinner_chars[idx % spinner_chars.len()],
                    bar,
                    pct * 100.0,
                    prog.done,
                    prog.total.max(1)
                );
                std::io::stdout().flush().ok();
            }
            idx += 1;
            thread::sleep(Duration::from_millis(100));
        }
        // Clear spinner line when done
        print!("\r{: <80}\r", "");
        std::io::stdout().flush().ok();
    });

    // Main thread: parse Cargo JSON output
    let reader = std::io::BufReader::new(stdout);
    let mut compiled_targets = HashSet::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(&line) {
            if let Some(reason) = v["reason"].as_str() {
                match reason {
                    "build-script-executed" => {
                        let mut prog = progress.lock().unwrap();
                        prog.total += 1;
                    }
                    "compiler-artifact" => {
                        if let Some(name) = v["target"]["name"].as_str() {
                            let mut prog = progress.lock().unwrap();
                            compiled_targets.insert(name.to_string());
                            prog.done = compiled_targets.len();
                            prog.total = prog.total.max(prog.done + 2); // heuristic
                        }
                    }
                    "build-finished" => {
                        let mut prog = progress.lock().unwrap();
                        prog.finished = true;
                    }
                    _ => {}
                }
            }
        }
    }

    let output = child.wait_with_output().context("waiting for cargo")?;
    spinner_handle.join().ok(); // wait for spinner thread

    if !output.status.success() {
        eprintln!("[cargo] build failed, stderr:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        anyhow::bail!("cargo build failed");
    }

    println!(
        "  [cargo] build finished ({}/{})",
        compiled_targets.len(),
        progress.lock().unwrap().total
    );

    Ok(())
}





// ------------------ Find the built executable ------------------
fn find_built_exe(project_dir: &Path, shim_name: &str, target: Option<&str>) -> Result<PathBuf> {
    let mut candidate = project_dir.join("target");
    if let Some(t) = target {
        candidate = candidate.join(t);
    }
    candidate = candidate.join("release").join(shim_name);
    #[cfg(windows)]
    {
        let with_exe = candidate.with_extension("exe");
        if with_exe.exists() {
            return Ok(with_exe);
        }
    }
    if candidate.exists() {
        return Ok(candidate);
    }
    anyhow::bail!("built shim binary not found: {:?}", candidate);
}

// ------------------ Write final exe = built_exe + zip + footer ------------------
fn write_final_exe(built_exe: &Path, zip_bytes: &[u8], out_path: &Path) -> Result<()> {
    println!("  [pack] reading built exe: {}", built_exe.display());
    let mut exe_bytes = Vec::new();
    File::open(built_exe)?.read_to_end(&mut exe_bytes)?;
    println!("  [pack] built exe size: {}", exe_bytes.len());
    let mut out = File::create(out_path)?;
    out.write_all(&exe_bytes)?;
    out.write_all(zip_bytes)?;
    let zip_size = zip_bytes.len() as u64;
    out.write_all(&zip_size.to_le_bytes())?;
    out.flush()?;
    println!("  [pack] wrote final exe at {} (zip {} bytes)", out_path.display(), zip_size);
    Ok(())
}

// ------------------ Main ------------------
fn main() -> Result<()> {
    let mut args: Vec<String> = env::args().collect();

    // ───────────────────────────────
    // 🧠 MODE DETECTION
    // ───────────────────────────────
    if args.len() == 1 {
        // 🟢 No args: default to current dir
        let cwd = env::current_dir()?;
        println!("[main] No args — defaulting to current directory: {}", cwd.display());
        return build_from_path(&cwd, None, None, None, None);
    }

    // Handle drag-and-drop of a single path (file or folder)
    if args.len() == 2 && !args[1].eq_ignore_ascii_case("build") {
        let dropped = PathBuf::from(&args[1]);
        if dropped.exists() {
            println!(
                "[main] Detected drag-and-drop: {} ({})",
                dropped.display(),
                if dropped.is_dir() { "folder" } else { "file" }
            );
            return build_from_path(&dropped, None, None, None, None);
        }
    }

    // ───────────────────────────────
    // 🛠️ Explicit "build" command mode
    // ───────────────────────────────
    if args.len() < 3 {
        usage_and_exit();
    }

    let cmd = args[1].as_str();
    if cmd != "build" {
        usage_and_exit();
    }

    let mec_folder = PathBuf::from(&args[2]);
    let mut shim_name = "mec_shim".to_string();
    let mut out_path = PathBuf::from("output.exe");
    let mut target: Option<String> = None;
    let mut cargo_flags: Option<String> = None;

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--shim-name" => { i += 1; shim_name = args[i].clone(); }
            "--out" => { i += 1; out_path = PathBuf::from(&args[i]); }
            "--target" => { i += 1; target = Some(args[i].clone()); }
            "--cargo-flags" => { i += 1; cargo_flags = Some(args[i].clone()); }
            other => {
                eprintln!("unknown arg: {}", other);
                usage_and_exit();
            }
        }
        i += 1;
    }

    build_from_path(&mec_folder, Some(shim_name), Some(out_path), target, cargo_flags)
}

fn build_from_path(
    input_path: &Path,
    shim_name: Option<String>,
    out_path: Option<PathBuf>,
    target: Option<String>,
    cargo_flags: Option<String>,
) -> Result<()> {
    let shim_name = shim_name.unwrap_or_else(|| "mec_shim".to_string());
    let out_path = out_path.unwrap_or_else(|| PathBuf::from("output.exe"));

    // Gather .mec files from either a folder or a single file
    let mut pairs = Vec::new();

    if input_path.is_file() {
        if input_path.extension().map(|s| s == "mec").unwrap_or(false) {
            println!("  [compile] reading {}", input_path.display());
            let stem = input_path.file_stem().unwrap().to_string_lossy().to_string();
            let bytes = compile_mec_to_bytecode(input_path)?;
            pairs.push((stem, bytes));
        }
    } else if input_path.is_dir() {
        println!("[main] building from folder: {}", input_path.display());
        for entry in fs::read_dir(&input_path).context("reading mec folder")? {
            let e = entry?;
            let path = e.path();
            if path.is_file() && path.extension().map(|s| s == "mec").unwrap_or(false) {
                println!("  [compile] reading {}", path.display());
                let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                let bytes = compile_mec_to_bytecode(&path)?;
                pairs.push((stem, bytes));
            }
        }
    }

    if pairs.is_empty() {
        anyhow::bail!("no .mec files found in provided path");
    }

    // create zip
    let zip_bytes = create_zip_from_pairs(&pairs)?;

    // create temp cargo project
    let temp = TempDir::new().context("creating tempdir")?;
    let project_dir = write_shim_project(&temp, &shim_name)?;
    println!("  [main] created shim project at {}", project_dir.display());

    // run cargo build with progress
    cargo_build_with_progress(&project_dir, target.as_deref(), cargo_flags.as_deref())?;

    // find built exe
    let built_exe = find_built_exe(&project_dir, &shim_name, target.as_deref())?;
    println!("[main] found built shim at {}", built_exe.display());

    // write final exe
    write_final_exe(&built_exe, &zip_bytes, &out_path)?;

    println!("[main] done. final exe at {}", out_path.display());
    Ok(())
}