#![allow(warnings)]
use std::{
  env,
  thread,
  time::{Duration, Instant},
  collections::{VecDeque, HashMap, HashSet},
  io::{stdout, Read, Write, Cursor, BufRead, BufReader},
  sync::{mpsc, Arc, OnceLock, Mutex, atomic::{AtomicBool, Ordering}},
  process::{Command as ProcessCommand, Stdio},
  path::{Path, PathBuf, MAIN_SEPARATOR},
  fs::{self, File},
};
use console::{style, Emoji};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use rand::prelude::IndexedRandom;
use rand::Rng;
use clap::{arg, command, value_parser, Arg, ArgAction, Command};
use colored::Colorize;
use tempfile::TempDir;
use anyhow::{Context, Result};
use serde_json::Value;

use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

use mech_syntax::*;
use mech_core::*;
use mech_interpreter::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");

static ERROR_MESSAGE: OnceLock<Arc<Mutex<Option<String>>>> = OnceLock::new();

static CANCELLED: OnceLock<Arc<AtomicBool>> = OnceLock::new();

static BUILD_DATA: OnceLock<Arc<Mutex<BuildData>>> = OnceLock::new();

static BUILD_DIR: &str = "./build";

#[derive(Debug, Default)]
pub struct BuildData {
  pub sources: Vec<String>,
  pub paths: Vec<PathBuf>,
  pub packages: Vec<String>,
  pub trees: Arc<Mutex<HashMap<String, Program>>>,
  pub bytecode: Vec<u8>,
  pub build_project_dir: Option<PathBuf>,
  pub temp_dir: Option<TempDir>,
  pub final_artifact: Option<PathBuf>,
  pub output_name: Option<String>,
}

fn init_cancel_flag() {
  CANCELLED.set(Arc::new(AtomicBool::new(false))).ok();
  ERROR_MESSAGE.set(Arc::new(Mutex::new(None))).ok();
  BUILD_DATA.set(Arc::new(Mutex::new(BuildData::default())));
}

fn set_output_name(path: impl Into<String>) {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.output_name = Some(path.into());
  } else {
    panic!("BuildData not initialized!");
  }
}

fn get_output_name() -> Option<String> {
  if let Some(data) = BUILD_DATA.get() {
    let data = data.lock().unwrap();
    data.output_name.clone()
  } else {
    None
  }
}

fn drop_temp_dir() {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.temp_dir.take();
  } else {
    panic!("BuildData not initialized!");
  }
}

fn get_final_artifact_path() -> Option<String> {
  if let Some(data) = BUILD_DATA.get() {
    let data = data.lock().unwrap();
    data.final_artifact.as_ref().map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()).display().to_string())
  } else {
    None
  }
}

fn set_final_artifact_path(path: impl Into<PathBuf>) {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.final_artifact = Some(path.into());
  } else {
    panic!("BuildData not initialized!");
  }
}

fn save_temp_dir(temp: TempDir) {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.temp_dir = Some(temp);
  } else {
    panic!("BuildData not initialized!");
  }
}

fn set_bytecode(bytes: Vec<u8>) {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.bytecode = bytes;
  } else {
    panic!("BuildData not initialized!");
  }
}

fn set_build_project_dir(path: impl Into<PathBuf>) {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.build_project_dir = Some(path.into());
  } else {
    panic!("BuildData not initialized!");
  }
}

pub fn get_build_project_dir() -> PathBuf {
  if let Some(data) = BUILD_DATA.get() {
    let data = data.lock().unwrap();
    data.build_project_dir.clone().unwrap().clone()
  } else {
    panic!("BuildData not initialized!");
  }
}

pub fn add_tree(path: impl Into<String>, tree: Program) {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.trees.lock().unwrap().insert(path.into(), tree);
  } else {
    panic!("BuildData not initialized!");
  }
}

pub fn get_trees() -> Arc<Mutex<HashMap<String, Program>>> {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.trees.clone()
  } else {
    panic!("BuildData not initialized!");
  }
}

pub fn get_build_data() -> Option<Arc<Mutex<BuildData>>> {
  BUILD_DATA.get().cloned()
}

pub fn add_path(path: impl Into<PathBuf>) {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.paths.push(path.into());
  } else {
    panic!("BuildData not initialized!");
  }
}

pub fn add_source(path: impl Into<String>) {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.sources.push(path.into());
  } else {
    panic!("BuildData not initialized!");
  }
}

pub fn get_sources() -> Vec<String> {
  BUILD_DATA
      .get()
      .map(|data| data.lock().unwrap().sources.clone())
      .unwrap_or_default()
}

fn cancel_all(msg: &str) {
  if let Some(flag) = CANCELLED.get() {
    flag.store(true, Ordering::Relaxed);
  }
  if let Some(err) = ERROR_MESSAGE.get() {
    *err.lock().unwrap() = Some(msg.to_string());
  }
}

fn is_cancelled() -> bool {
  CANCELLED.get()
    .map(|f| f.load(Ordering::Relaxed))
    .unwrap_or(false)
}

fn get_error_message() -> Option<String> {
  ERROR_MESSAGE.get()
    .and_then(|err| err.lock().unwrap().clone())
}

static EMPTY: &[&str] = &[" "," "];

static SAND: &[&str] = &["⠁","⠂","⠄","⡀","⡈","⡐","⡠","⣀","⣁","⣂","⣄","⣌","⣔","⣤","⣥","⣦","⣮","⣶","⣷","⣿","⡿","⠿","⢟","⠟","⡛","⠛","⠫","⢋","⠋","⠍","⡉","⠉","⠑","⠡","⢁","⣿"];

static FISTBUMP: &[&str] = &[
  "   🤜　　　　🤛 ",
  "   🤜　　　　🤛 ",
  "   🤜　　　　🤛 ",
  "   　🤜　　🤛　 ",
  "   　　🤜🤛　　 ",
  "   　🤜💥🤛　　 ",
  "    🤜 ✨ 🤛　 ",
  "   🤜　💭 　🤛　 ",
  "   ✌️　　　　✌️ ",
  "   ✌️　　　　✌️ ",
  "   ✌️　　　　✌️ ",
  " "
];

static DOTSPINNER: &[&str] = &[
  "∙∙∙","∙∙∙","●∙∙","●∙∙","•●∙","•●∙","∙•●","∙•●","∙∙•","∙∙•","∙∙∙","∙∙∙","∙∙∙","∙∙∙","∙∙∙"," "
];

static MINDBLOWN: &[&str] = &[
  "🙂","🙂","😐","😐","😮","😮","😦","😦","😧","😧","🤯","🤯","💥","✨","💭","　","　",
];

static SQUARESPINNER: &str = "◰◰◳◳◲◲◱◱▣";

static FAILEDSQUARESPINNER: &str = "▨▨";

static PENDINGSQUARESPINNER: &str = "□□";

static PARALLELOGRAMPROGRESS: &str = "▰▱";

fn format_duration_short(dur: Duration) -> String {
  let ms = dur.as_millis();
  if ms < 1000 {
    format!("{ms}ms")
  } else if ms < 60_000 {
    format!("{:.1}s", ms as f64 / 1000.0)
  } else if ms < 3_600_000 {
    let secs = ms / 1000;
    let mins = secs / 60;
    let rem = secs % 60;
    format!("{mins}m{rem}s")
  } else {
    let secs = ms / 1000;
    let hrs = secs / 3600;
    let mins = (secs % 3600) / 60;
    format!("{hrs}h{mins}m")
  }
}

#[derive(Debug)]
enum StepStatus {
    NotStarted,
    Pending,
    Running,
    Completed,
    Failed,
}

struct BuildProcess {
  id: u64,
  name: String,
  build_status: StepStatus,
  indicators: MultiProgress,
  stage_handles: Vec<thread::JoinHandle<()>>,
  start_time: Option<Instant>,
  end_time: Option<Instant>,
  stages: VecDeque<BuildStage>,
  build_progress: ProgressBar,
  status_bar: ProgressBar,
  final_build_location: Option<PathBuf>,
}

impl BuildProcess {

  pub fn new(id: u64, name: String) -> Self {
    let progress_style = ProgressStyle::with_template(
      "{prefix:.yellow} {bar:60.yellow/white.dim.bold} {percent}% ({pos}/{len})"
    ).unwrap()
     .progress_chars(PARALLELOGRAMPROGRESS);
    let m = MultiProgress::new();
    let build_progress = m.add(ProgressBar::new(0));
    build_progress.set_style(progress_style);
    build_progress.set_prefix("[Build]");

    // The status bar will be a spinner and it will also show the elapsed time
    let status_style = ProgressStyle::with_template(
      "{spinner:.yellow} [{elapsed_precise}] {msg}"
    ).unwrap()
     .tick_strings(SAND);
    let status_bar = m.add(ProgressBar::new_spinner());
    status_bar.set_style(status_style);
    status_bar.enable_steady_tick(Duration::from_millis(100));
    status_bar.set_message("");

    BuildProcess {
      id, name,
      build_progress,
      status_bar,
      indicators: m,
      build_status: StepStatus::NotStarted,
      stage_handles: Vec::new(),
      start_time: None,
      end_time: None,
      stages: VecDeque::new(),
      final_build_location: None,
    }
  }

  pub fn start(&mut self) {
    self.start_time = Some(Instant::now());
    self.build_status = StepStatus::Running;
    self.status_bar.set_message("Starting build...");
  }

  pub fn finish(&mut self) {
    self.status_bar.finish_and_clear();
    let final_state = self.indicators.add(ProgressBar::new_spinner());
    self.end_time = Some(Instant::now());
    self.build_status = StepStatus::Completed;
    self.build_progress.finish_and_clear();
    let completed_style = ProgressStyle::with_template(
      "{prefix:.yellow} {msg} {spinner}"
    ).unwrap()
      .tick_strings(FISTBUMP);  
    final_state.set_style(completed_style);
    final_state.set_prefix("[Success]");
    // Run the fistbump animation
    if let Some(start_time) = self.start_time {
      let elapsed = self.end_time.unwrap_or_else(Instant::now).duration_since(start_time);
      if elapsed > Duration::from_secs(60) {
        for _ in 0..FISTBUMP.len() - 1 {
          thread::sleep(Duration::from_millis(100));
          final_state.tick();
        }
      }
    }
    let path = get_final_artifact_path().unwrap_or_else(|| "Unknown".to_string());
    final_state.finish_with_message(format!("Artifact available at: {}", style(path).color256(75)));
  }

  pub fn fail(&mut self) {
    self.status_bar.finish_and_clear();
    let final_state = self.indicators.add(ProgressBar::new_spinner());
    self.end_time = Some(Instant::now());
    self.build_status = StepStatus::Failed;
    self.build_progress.finish_and_clear();
    let failed_style = ProgressStyle::with_template(
      "{prefix:.yellow} {msg} {spinner}"
    ).unwrap()
      .tick_strings(MINDBLOWN);
    final_state.set_style(failed_style);
    final_state.set_prefix("[Failure]");
    // Run the fistbump animation
    for _ in 0..MINDBLOWN.len() - 1 {
      thread::sleep(Duration::from_millis(100));
      final_state.tick();
    }
    final_state.finish_with_message(get_error_message().unwrap_or("Unknown error".to_string()));
  }

  pub fn add_build_stage(&mut self, mut stage: BuildStage, steps: usize) {
    match self.build_status {
      StepStatus::NotStarted => self.start(),
      _ => {}
    }
    let stage_id = self.stages.len() as u64 + 1;
    
    // Apply Header To Section
    let header_style = ProgressStyle::with_template(
      "{prefix:.yellow.dim} {msg:.dim} {spinner:.dim}"
    ).unwrap()
     .tick_strings(&EMPTY);
    let header = self.indicators.insert_before(&self.build_progress, ProgressBar::new_spinner());
    header.set_style(header_style);
    header.set_prefix(format!("{}❱", stage_id));
    header.set_message(stage.name.clone());

    stage.id = stage_id;
    stage.header = header.clone(); 
    stage.last_step = header.clone();
    stage.build_progress = self.build_progress.clone();
    stage.indicators = Some(self.indicators.clone());
    stage.status = StepStatus::Pending;
    self.stages.push_back(stage);

    self.build_progress.inc_length(1 + steps as u64);   
  }

}

struct BuildStage {
  pub id: u64,
  name: String,
  status: StepStatus,
  start_time: Option<Instant>,
  end_time: Option<Instant>,
  //steps: Vec<BuildStep>,
  header: ProgressBar,
  pub last_step: ProgressBar,
  style: ProgressStyle,
  pub build_progress: ProgressBar,
  pub indicators: Option<MultiProgress>,
  task_fn: Option<Box<dyn FnOnce(&mut BuildStage) + Send + 'static>>,
}

impl BuildStage {
  pub fn new<F>(name: impl Into<String>, f: F) -> Self
  where
      F: FnOnce(&mut BuildStage) + Send + 'static,
  {
    let style = ProgressStyle::with_template("{prefix:.yellow} {msg} {spinner:.dim}")
        .unwrap()
        .tick_strings(&DOTSPINNER);

    Self {
      id: 0,
      name: name.into(),
      status: StepStatus::NotStarted,
      start_time: None,
      end_time: None,
      header: ProgressBar::new(0),
      last_step: ProgressBar::new(0),
      style,
      build_progress: ProgressBar::new(0),
      indicators: None,
      task_fn: Some(Box::new(f)),
    }
  }

  pub fn start(&mut self) {
    self.start_time = Some(Instant::now());
    self.status = StepStatus::Running;
    if is_cancelled() {
      self.cancel();
      return;
    }
    self.header.set_style(self.style.clone());
    self.header.enable_steady_tick(Duration::from_millis(100));
    self.task_fn.take().map(|f| f(self));
    if is_cancelled() {
      self.fail();
    } else {
      self.finish();
    }
  }

  pub fn finish(&mut self) {
    let end_time = Instant::now();
    self.end_time = Some(end_time);
    self.status = StepStatus::Completed;
    self.build_progress.inc(1);
    let elapsed = self.end_time.unwrap().duration_since(self.start_time.unwrap());
    let formatted_time = format_duration_short(elapsed);
    self.header.finish_with_message(format!("{} [{}] {}", self.name, formatted_time, style("✓").green()));
  }

  pub fn fail(&mut self) {
    self.end_time = Some(Instant::now());
    self.status = StepStatus::Failed;
    self.header.finish_with_message(format!("{} {}", self.name, style("✗").red()));
  }

  pub fn cancel(&mut self) {
    self.end_time = Some(Instant::now());
    self.status = StepStatus::Failed;
    let cancel_style = ProgressStyle::with_template(
      "{prefix:.yellow.dim} {msg:.dim}"
    ).unwrap()
     .tick_chars(FAILEDSQUARESPINNER);
    self.header.set_style(cancel_style);
    self.header.finish_with_message(format!("{} {}", self.name, "✗"));
  }

}

pub fn main() -> anyhow::Result<()> {
  let text_logo = r#"
  ┌─────────┐ ┌──────┐ ┌─┐ ┌──┐ ┌─┐  ┌─┐
  └───┐ ┌───┘ └──────┘ │ │ └┐ │ │ │  │ │
  ┌─┐ │ │ ┌─┐ ┌──────┐ │ │  └─┘ │ └─┐│ │
  │ │ │ │ │ │ │ ┌────┘ │ │  ┌─┐ │ ┌─┘│ │
  │ │ └─┘ │ │ │ └────┐ │ └──┘ │ │ │  │ │
  └─┘     └─┘ └──────┘ └──────┘ └─┘  └─┘"#.truecolor(246,192,78);
  let about = format!("{}", text_logo);

  let start_time = Instant::now();
  init_cancel_flag();
  let mut args: Vec<String> = env::args().collect();

  let matches = Command::new("Mech")
    .version(VERSION)
    .author("Corey Montella corey@mech-lang.org")
    .about(about)
    .arg(Arg::new("mech_paths")
        .help("Source .mec and files")
        .required(false)
        .action(ArgAction::Append))
    .arg(Arg::new("debug")
        .short('d')
        .long("debug")
        .help("Print debug info")
        .action(ArgAction::SetTrue))
    .arg(Arg::new("release")
        .short('r')
        .long("release")
        .help("Build in release mode")
        .action(ArgAction::SetTrue))        
    .arg(Arg::new("output_name")
        .short('o')
        .long("out")
        .help("Name out output artifact.")
        .required(false))    
    .subcommand(Command::new("clean")
      .about("Clean the build artifacts.")
      .arg(Arg::new("mech_clean_paths")
        .help("Source .mec and .mecb files")
        .required(false)
        .action(ArgAction::Append)))
    .subcommand(Command::new("build")
      .about("Build Mech program into a binary.")
      .arg(Arg::new("mech_build_file_paths")
        .help("Source .mec and .mecb files")
        .required(false)
        .action(ArgAction::Append))
      .arg(Arg::new("build_debug")
        .short('d')
        .long("debug")
        .help("Print debug info")
        .action(ArgAction::SetTrue))        
      .arg(Arg::new("build_release")
        .short('r')
        .long("release")
        .help("Build in release mode")
        .action(ArgAction::SetTrue))       
      .arg(Arg::new("build_output_name")
        .short('o')
        .long("out")
        .help("Name of output artifact.")
        .required(false)))            
    .get_matches();

  if let Some(matches) = matches.subcommand_matches("clean") {
    let clean_paths: Vec<String> = matches.get_many::<String>("mech_clean_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    if clean_paths.is_empty() {
      // Clean the default build directory
      if Path::new(BUILD_DIR).exists() {
        fs::remove_dir_all(BUILD_DIR).with_context(|| format!("Failed to remove build directory: {}", BUILD_DIR))?;
        println!("{} Cleaned build directory: {}", style("✓").green(), BUILD_DIR);
      } else {
        println!("{} Build directory does not exist: {}", style("✗").red(), BUILD_DIR);
      }
    } else {
      for path in clean_paths {
        let p = Path::new(&path);
        if p.exists() {
          if p.is_dir() {
            fs::remove_dir_all(p).with_context(|| format!("Failed to remove directory: {}", path))?;
            println!("{} Cleaned directory: {}", style("✓").green(), path);
          } else if p.is_file() {
            fs::remove_file(p).with_context(|| format!("Failed to remove file: {}", path))?;
            println!("{} Cleaned file: {}", style("✓").green(), path);
          } else {
            println!("Path is neither file nor directory: {}", path);
          }
        } else {
          println!("{} Path does not exist: {}", style("✗").red(), path);
        }
      }
    }
    return Ok(());
  }

  let mut debug_flag = matches.get_flag("debug");
  let mut release_flag = matches.get_flag("release");
  let mut mech_paths: Vec<String> = matches.get_many::<String>("mech_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
  let mut output_name = matches.get_one::<String>("output_name").map(|s| s.to_string()).unwrap_or("program".to_string());

  if let Some(matches) = matches.subcommand_matches("build") {
    mech_paths = matches.get_many::<String>("mech_build_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    debug_flag = matches.get_flag("build_debug");
    release_flag = matches.get_flag("build_release");
    output_name = matches.get_one::<String>("build_output_name").map(|s| s.to_string()).unwrap_or("program".to_string());
  }

  set_output_name(output_name.clone());

  if mech_paths.is_empty() {
    // use the current directory
    args.push(".".to_string());
    let current_dir = env::current_dir().unwrap();
    add_source(current_dir.to_str().unwrap());
    println!("{} No source files provided, using current directory: {}", style("✗").red(), current_dir.display());
  } else {
    for path in &mech_paths {
      add_source(path);
    }
  }

  let start_message = if mech_paths.len() == 1 {
    format!("{}", mech_paths[0])
  } else {
    format!("{} files.", mech_paths.len())
  };

  // Start the build!
  println!(r#"{} Building: {}"#, style(format!("[mech v{}]",VERSION).yellow()), start_message);
  {
    let mut build = BuildProcess::new(42, "Mech Builder".to_string());
    let m = build.indicators.clone();
    let cancelled = Arc::new(AtomicBool::new(false));

    let (path_tx, path_rx) = mpsc::channel();
    let (tree_tx, tree_rx) = mpsc::channel();

    let path_tx2 = path_tx.clone();
    let mut prepare_build = BuildStage::new("Prepare build environment", |mut stage| {
      prepare_build(&mut stage, path_tx2);
    });

    let mut download_packages = BuildStage::new("Download packages", |mut stage| {
      download_packages(&mut stage,path_tx);
    });

    let mut build_packages = BuildStage::new("Build packages", |mut stage| {
      parse_packages(&mut stage,path_rx, tree_tx);
    });

    let mut build_project = BuildStage::new("Build project", |mut stage| {
      build_project(&mut stage, tree_rx);
    });

    let mut compile_shim = BuildStage::new("Compile shim", move |mut stage| {
      compile_shim(&mut stage, release_flag);
    });

    let mut package_artifacts = BuildStage::new("Package artifacts", move |mut stage| {
      package_artifacts(&mut stage, release_flag);
    });

    let status = build.status_bar.clone();

    build.add_build_stage(prepare_build, 5);
    build.add_build_stage(download_packages, 0);
    build.add_build_stage(build_packages, 0);
    build.add_build_stage(build_project, 0);
    build.add_build_stage(compile_shim, 145);
    build.add_build_stage(package_artifacts, 3);
    
    // Stage 1 - Prepare the build environment
    status.set_message("Preparing environment...");
    let mut prepare_environment_stage = build.stages.pop_front().unwrap();
    prepare_environment_stage.start();

    // Stage 2 and 3 - Download and build packages
    status.set_message("Downloading and building packages...");
    let mut download_stage = build.stages.pop_front().unwrap();
    let jh1 = thread::spawn(move || {
      download_stage.start();
    });
    
    let mut build_stage_1 = build.stages.pop_front().unwrap();
    let jh2 = thread::spawn(move || {
      build_stage_1.start();
    });
    jh1.join();
    jh2.join();

    // Stage 4 - Build the project
    status.set_message("Building project...");
    let mut build_stage_2 = build.stages.pop_front().unwrap();
    build_stage_2.start();

    // Stage 5 - Compile the Rust shim
    status.set_message("Compiling shim...");
    let mut compile_shim_stage = build.stages.pop_front().unwrap();
    compile_shim_stage.start();

    // Stage 6 - Package the executable (shim + zipped bytecode)
    status.set_message("Packaging executable...");
    let mut packaging_stage = build.stages.pop_front().unwrap();
    packaging_stage.start();
    if is_cancelled() {
      build.fail();
    } else {
      build.finish();
    }
  }
  let end_time = Instant::now();
  let elapsed = end_time.duration_since(start_time);
  let formatted_time = format_duration_short(elapsed);
  if is_cancelled() {
    println!("{} Build failed after {}.", style("✗").red(), formatted_time);
  } else {
    println!("{} Build succeeded after {}!", style("✓").green(), formatted_time);
  }


  drop_temp_dir();
  Ok(())
}

// This is a step-bt-step process:
// 0. Create a /build directory where we will put all the build artifacts
// 1. Open supplied source files, gather all the files that are contained
//    (File types are just aliases for compiler feature sets)
//    .mec     (Mech source)
//    .mpkg    (Mech package file)
//    .mecb    (Mech binary file)
//    .mdoc    (Mechdown file)
//    .mdb     (Mech database file)
//    .dll     (Dynamic library)
//    .rlib    (Rust library)
//    .m file  (MATLAB)
// 2. Start by looking for a index.mpkg file in the root of the project
// 3. Parse the .mpkg file, get the name of the project and the version of mech we are targeting.
// 4. Verify the version of mech is compatible with the current version.
// 5. Prepare the build evnvironment:
//    a. Create a /build directory if it doesn't exist
//    b. Configure the build directory according to the project settings
//    c. Set up any environment variables that are required

fn prepare_build(stage: &mut BuildStage, tx: mpsc::Sender<Vec<PathBuf>>) {
  let m = stage.indicators.as_ref().unwrap().clone();
  let build_progress = stage.build_progress.clone();

  let mut steps = VecDeque::new();

  // 0. Create a /build directory
  let pb = m.insert_after(&stage.last_step,ProgressBar::new_spinner());
  stage.last_step = pb.clone();
  pb.set_style(build_style());
  pb.set_message("Create build directory.");
  steps.push_back(pb);

  // 1. Open supplied source files
  let pb = m.insert_after(&stage.last_step,ProgressBar::new_spinner());
  stage.last_step = pb.clone();
  pb.set_style(pending_style());
  pb.set_message("Gather source files.");
  steps.push_back(pb);

  // 2. Look for index.mpkg file in root of project
  let pb = m.insert_after(&stage.last_step,ProgressBar::new_spinner());
  stage.last_step = pb.clone();
  pb.set_style(pending_style());
  pb.set_message("Check for index.mpkg file.");
  steps.push_back(pb);

  // 4. Verify mech version compatibility
  let pb = m.insert_after(&stage.last_step,ProgressBar::new_spinner());
  stage.last_step = pb.clone();
  pb.set_style(pending_style());
  pb.set_message("Verify mech version compatibility.");
  steps.push_back(pb);

  // 5. Prepare build environment
  let pb = m.insert_after(&stage.last_step,ProgressBar::new_spinner());
  stage.last_step = pb.clone();
  pb.set_style(pending_style());
  pb.set_message("Prepare build environment.");
  steps.push_back(pb);
  
  // Step 0
  let step = steps.pop_front().unwrap();
  let build_path = Path::new(BUILD_DIR);
  if !build_path.exists() {
    if let Err(e) = fs::create_dir_all(build_path) {
      step.finish_with_message(format!("Failed to create build directory: {} {}",e,style("✗").red()));
      stage.fail();
      return;
    }
  }
  step.finish_with_message("Build directory ready"); 
  build_progress.inc(1);

  // Step 1 
  let step = steps.pop_front().unwrap();
  step.set_style(build_style());
  let exts = ["mec", "mpkg", "mecb", "mdoc", "mdb", "dll", "rlib", "m", "md"];
  let sources = get_sources();
  for src in sources {
    let path = Path::new(&src);
    if path.exists() {
      if let Err(e) = gather_source_files(path, &exts) {
        step.finish_with_message(format!("Failed reading {}: {} {}", src, e, style("✗").red()));
        cancel_all("Build cancelled due to IO error.");
        stage.fail();
        return;
      }
    } else {
      step.finish_with_message(format!("Source path does not exist: {} {}", src, style("✗").red()));
      cancel_all("Build cancelled due to missing source files.");
      stage.fail();
      return;
    }
  }
  let source_files = get_build_data().unwrap().lock().unwrap().paths.clone();
  tx.send(source_files.clone());
  let file_count = source_files.len();
  let file_label = if file_count == 1 { "file" } else { "files" };
  step.finish_with_message(format!("Discovered {} source {}.", file_count, file_label));
  build_progress.inc(1);

  // Step 2
  let step = steps.pop_front().unwrap();
  step.set_style(build_style());
  let mut found_index = false;
  for src in &source_files {
    if let Some(fname) = src.file_name() {
      if fname == "index.mpkg" {
        found_index = true;
        break;
      }
    }
  }
  if found_index {
    step.finish_with_message(format!("Found index.mpkg file {}", style("✓").green()));
    // 3. Parse the .mpkg file
    let pb = m.insert_after(&step,ProgressBar::new_spinner());
    pb.set_style(build_style());
    pb.set_message("Parsing index.mpkg.");
    todo!("Parse the .mpkg file");
  } else {
    step.finish_with_message(format!("No index.mpkg file {}", style("🛈").color256(75)));
  }
  build_progress.inc(1);

  // Step 4
  let step = steps.pop_front().unwrap();
  step.set_style(build_style());
  step.finish_with_message(format!("Targeting Mech v{}.", VERSION));
  build_progress.inc(1);

  // Step 5 - Configure environment
  // todo: read the index file and configure the environment accordingly
  let step = steps.pop_front().unwrap();
  step.set_style(build_style());
  step.finish_with_message(format!("Configured build environment"));
  build_progress.inc(1);
}

fn download_packages(stage: &mut BuildStage, tx: mpsc::Sender<Vec<PathBuf>>) {
  let m = stage.indicators.as_ref().unwrap().clone();
  let build_progress = stage.build_progress.clone();
  /*let mut handles = Vec::new();
  for pkg in PACKAGES {
    // Random size per package
    build_progress.inc_length(1);
    let pb = m.insert_after(&stage.header, ProgressBar::new(rand_size));
    pb.set_style(download_style());
    pb.set_prefix("  ");
    pb.set_message(format!("{:<20}", pkg));

    let tx = tx.clone();

    let build_progress = stage.build_progress.clone();
    let handle = thread::spawn(move || {
      for j in 0..=rand_size {
        if is_cancelled() {
          pb.set_style(fail_style());
          pb.finish();
          return;
        }
        pb.set_position(j);
        thread::sleep(Duration::from_millis(20 + rand::thread_rng().gen_range(0..300)));
        // with 5% probability, fail the download
        if rand::thread_rng().gen_range(0..5000) < 1 && j > 20 && j < rand_size - 20 {
          pb.set_style(fail_style());
          pb.finish_with_message(format!("{:<20} {}", pkg, style("✗").red()));
          build_progress.inc(1);
          cancel_all(format!("Failed to download package: {}", pkg).as_str());
          return;
        }
      }
      pb.finish_and_clear();
      let _ = tx.send(pkg.clone().to_string());
      build_progress.inc(1);
    });

    handles.push(handle);
  }

  for handle in handles {
    let _ = handle.join();
  }
  drop(tx);*/
}

fn parse_packages(stage: &mut BuildStage, rx: mpsc::Receiver<Vec<PathBuf>>, tx: mpsc::Sender<Program>) {
  let m = stage.indicators.as_ref().unwrap().clone();
  let build_progress = stage.build_progress.clone();

  let mut handles = Vec::new();
  
  // Read from the channel and spawn build tasks until the channel is closed
  for pkgs in rx {
    build_progress.inc_length(1);
    let pb = m.insert_after(&stage.header, ProgressBar::new_spinner());
    pb.set_style(build_style());
    pb.set_message("Building project:...");
    pb.enable_steady_tick(Duration::from_millis(100));
    
    let build_progress = build_progress.clone();
    
    let handle = thread::spawn(move || {
      for pkg in pkgs {
        pb.set_message(format!("{:<20}", pkg.display()));
        if is_cancelled() {
          pb.set_style(fail_style());
          pb.finish();
          return;
        }
        // Open the file
        let content = match fs::read_to_string(&pkg) {
          Ok(c) => c,
          Err(e) => {
            pb.set_style(fail_style());
            pb.finish_with_message(format!("Failed to read {}: {} {}", pkg.display(), e, style("✗").red()));
            cancel_all("Build cancelled due to IO error.");
            return;
          }
        };
        // Parse the file
        let tree = parser::parse(&content);
        match tree {
          Ok(t) => {
            add_tree(pkg.display().to_string(), t);
            pb.set_message(format!("{}", pkg.display()));
          }
          Err(e) => {
            pb.set_style(fail_style());
            pb.finish_with_message(format!("Failed to parse {}: {:?} {}", pkg.display(), e, style("✗").red()));
            cancel_all("Build cancelled due to parse error.");
            return;
          }
        }
      }
      pb.finish();
      build_progress.inc(1);
    });
    handles.push(handle);
  }

  for handle in handles {
    let _ = handle.join();
  }

  drop(tx);

}

fn build_project(stage: &mut BuildStage, rx: mpsc::Receiver<Program>) {
  let m = stage.indicators.as_ref().unwrap().clone();
  let build_progress = stage.build_progress.clone();

  //let mut handles = Vec::new();
  
  // Read from the channel and spawn build tasks until the channel is closed
  build_progress.inc_length(1);
  let pb = m.insert_after(&stage.header, ProgressBar::new_spinner());
  pb.set_style(build_style());
  pb.set_message("Building project:...");
  pb.enable_steady_tick(Duration::from_millis(100));
  
  let mut intrp = Interpreter::new(0);

  for tree in rx {
    let result = intrp.interpret(&tree);
    match result {
      Ok(_) => {
        pb.set_message("Build succeeded.");
      }
      Err(e) => {
        pb.set_style(fail_style());
        pb.finish_with_message(format!("Build failed: {:?} {}", e, style("✗").red()));
        cancel_all("Build cancelled due to interpreter error.");
        return;
      }
    }
  }

  match intrp.compile() {
    Ok(bytecode) => {
      pb.set_message(format!("Compiled {} bytes.", bytecode.len()));
      set_bytecode(bytecode);
    }
    Err(e) => {
      pb.set_style(fail_style());
      pb.finish_with_message(format!("Compilation failed: {:?} {}", e, style("✗").red()));
      cancel_all(format!("Build cancelled due to compile error: {:?}", e).as_str());
      return;
    }
  }

  pb.finish();
  build_progress.inc(1);
}

// Helpers

pub fn short_source_name(path: &str) -> String {
  Path::new(path)
      .file_name()
      .and_then(|n| n.to_str())
      .unwrap_or(path)
      .to_string()
}

fn build_style() -> ProgressStyle {
  ProgressStyle::with_template(
    "   {spinner:.yellow} {msg}",
  ).unwrap()
    .tick_chars(SQUARESPINNER)
}

fn fail_style() -> ProgressStyle {
  ProgressStyle::with_template(
    "   {spinner:.red} {msg}",
  ).unwrap()
    .tick_chars(FAILEDSQUARESPINNER)
}

fn pending_style() -> ProgressStyle {
  ProgressStyle::with_template(
    "   {spinner:.dim} {msg:.dim}",
  ).unwrap()
    .tick_chars(PENDINGSQUARESPINNER)
}

fn download_style() -> ProgressStyle {
  ProgressStyle::with_template(
    "   {spinner:.yellow} {msg} {bar:20.yellow/white.dim.bold} {percent}%",
  ).unwrap()
    .progress_chars(PARALLELOGRAMPROGRESS)
    .tick_chars(SQUARESPINNER)
}

fn pending_download_style() -> ProgressStyle {
  ProgressStyle::with_template(
    "   {spinner:.yellow} {msg} {bar:20.yellow/white.dim.bold} {percent}%",
  ).unwrap()
    .progress_chars(PARALLELOGRAMPROGRESS)
    .tick_chars(PENDINGSQUARESPINNER)
}

fn gather_source_files(path: &Path, exts: &[&str]) -> std::io::Result<()> {
  if path.is_file() {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if exts.contains(&ext) {
          add_path(path.to_path_buf());
        }
    }
  } else if path.is_dir() {
    for entry in fs::read_dir(path)? {
      let entry = entry?;
      gather_source_files(&entry.path(), exts)?;
    }
  }
  Ok(())
}

fn compile_shim(stage: &mut BuildStage, release: bool) {
  let m = stage.indicators.as_ref().unwrap().clone();

  let pb = m.insert_after(&stage.last_step,ProgressBar::new_spinner());
  stage.last_step = pb.clone();
  pb.set_style(build_style());
  pb.set_message("Create temp directory for shim project.");
  pb.enable_steady_tick(Duration::from_millis(100));

  // create temp cargo project
  let temp = match TempDir::new().context("creating tempdir") {
    Ok(t) => {
      set_build_project_dir(t.path().to_path_buf());
      t
    },
    Err(e) => {
      pb.set_style(fail_style());
      pb.finish_with_message(format!("Failed to create temp dir: {} {}", e, style("✗").red()));
      cancel_all("Build cancelled due to IO error.");
      stage.fail();
      return;
    }
  };
  let project_dir = match write_shim_project(&temp, &"mech_shim") {
    Ok(p) => p,
    Err(e) => {
      pb.set_style(fail_style());
      pb.finish_with_message(format!("Failed to write shim project: {} {}", e, style("✗").red()));
      cancel_all("Build cancelled due to IO error.");
      stage.fail();
      return;
    }
  };
  pb.finish_with_message(format!("Created temp shim project at {}", project_dir.canonicalize().unwrap_or_else(|_| project_dir.clone()).display()));

  save_temp_dir(temp);

  match cargo_build(&project_dir, release, stage) {
    Ok(()) => {
    }
    Err(e) => {
      stage.fail();
      return;
    }
  }
  stage.build_progress.inc(1);
}

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

mech-core = {{version = "{version}", default-features = false, features = ["base"] }}
mech-interpreter = {{version = "{version}", default-features = false, features = ["base"] }}
"#,
      name = shim_name,
      version = VERSION
  );
  fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;
  // src/main.rs -- shim reads appended ZIP using the footer (last 8 bytes)
  let main_rs = r#"
use anyhow::{Result, Context};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Cursor};
use zip::read::ZipArchive;

use mech_core::*;
use mech_interpreter::*;

fn run_bytecode(name: &str, bytecode: &[u8]) -> MResult<Value> {
  let mut intrp = Interpreter::new(0);
  match ParsedProgram::from_bytes(&bytecode) {
    Ok(prog) => {
      intrp.run_program(&prog)
    },
    Err(e) => {
        return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("{:?}", e), id: line!(), kind: MechErrorKind::GenericError("Unknown".to_string())});
    }
  }
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
    let result = run_bytecode(&name, &data);
    println!("[shim] result: {:?}", result);
  }
  println!("[shim] Press Enter to exit...");
  std::io::stdin().read_line(&mut String::new()).unwrap();
  Ok(())
}
"#;
  fs::write(project_dir.join("src").join("main.rs"), main_rs)?;
  Ok(project_dir)
}

pub fn cargo_build(
  project_dir: &Path,
  //target: Option<&str>,
  //extra_flags: Option<&str>,
  release: bool,
  stage: &mut BuildStage,
) -> MResult<()> {
  let m = stage.indicators.as_ref().unwrap().clone();
  let build_progress = stage.build_progress.clone();
  let header = stage.header.clone();
  header.set_message("Building shim");
  
  let pb = m.insert_after(&stage.last_step,ProgressBar::new_spinner());
  stage.last_step = pb.clone();
  pb.set_style(build_style());
  pb.set_message("Running shim build...");
  pb.enable_steady_tick(Duration::from_millis(100));

  let mut cmd = ProcessCommand::new("cargo");

  // get the current working directory for the app
  let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
  // extend path to build dir
  let build_dir = current_dir.join(BUILD_DIR);

  cmd.current_dir(project_dir)
      .arg("build")
      .env("CARGO_TARGET_DIR", build_dir)
      .arg("--message-format=json")
      .stdout(Stdio::piped())
      .stderr(Stdio::piped());

  if release {
    cmd.arg("--release");
  }

  /*if let Some(t) = target {
    cmd.arg("--target").arg(t);
  }

  if let Some(flags) = extra_flags {
    for tok in flags.split_whitespace() {
      cmd.arg(tok);
    }
  }*/

  let mut child = match cmd.spawn().context("failed to spawn shim build") {
    Ok(c) => c,
    Err(e) => {
      pb.set_style(fail_style());
      pb.finish_with_message(format!("Failed to start cargo: {} {}", e, style("✗").red()));
      cancel_all("Build cancelled due to cargo error.");
      return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Failed to start cargo: {}", e), id: line!(), kind: MechErrorKind::GenericError("Cargo build failed".to_string())});
    }
  };
  let stdout = child.stdout.take().unwrap();

  let reader = std::io::BufReader::new(stdout);

  let mut finished_targets = HashSet::new();

  for line in reader.lines() {
    let line = line?;
    if line.trim().is_empty() {
      continue;
    }

    if let Ok(v) = serde_json::from_str::<Value>(&line) {
      if let Some(reason) = v["reason"].as_str() {
        let target_name = v["target"]["name"].as_str().unwrap_or("");
        match reason {
          "compiler-artifact" => {
            let target_name = v["target"]["name"].as_str().unwrap_or("unknown");
            let version = v["package_id"].as_str().unwrap_or("0.0.0");
            // get version from package ID, it's after the @ symbol at the end
            let version = version.split('@').last().unwrap_or("0.0.0");
            finished_targets.insert(target_name.to_string());
            pb.set_message(format!("Compiled target: {} v{}", target_name, version));
            build_progress.inc(1);
          }
          "build-finished" => {

          }
          x => {
            //pb.set_message(format!("Cargo: {:?}", x));
          }
        }
      }
    }
  }

  match child.wait_with_output().context("waiting for cargo") {
    Ok(output) => {
      if !output.status.success() {
        pb.set_style(fail_style());
        pb.finish_with_message(format!("Shim build failed: {} {} {}", output.status, String::from_utf8_lossy(&output.stderr), style("✗").red()));
        cancel_all("Build cancelled due to cargo error.");
        return Err(MechError {file: file!().to_string(), tokens: vec![], msg: "Cargo build failed".to_string(), id: line!(), kind: MechErrorKind::GenericError("Cargo build failed".to_string())});
      } else {
        pb.finish_with_message("Shim build finished.");
      }
    },
    Err(e) => {
      pb.set_style(fail_style());
      pb.finish_with_message(format!("Failed to wait for Cargo: {} {}", e, style("✗").red()));
      cancel_all("Build cancelled due to Cargo error.");
      return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Failed to wait for Cargo: {}", e), id: line!(), kind: MechErrorKind::GenericError("Cargo build failed".to_string())});
    }
  }


  Ok(())
}

fn package_artifacts(stage: &mut BuildStage, release: bool) {
  let m = stage.indicators.as_ref().unwrap().clone();
  let header = stage.header.clone();
  header.set_message("Packaging artifacts");
  let build_progress = stage.build_progress.clone();

  let mut steps = VecDeque::new();

  // zip bytecode
  let pb = m.insert_after(&stage.last_step,ProgressBar::new_spinner());
  stage.last_step = pb.clone();
  pb.set_style(pending_style());
  pb.set_message("Compress bytecode.");
  steps.push_back(pb.clone());

  // read shim
  let pb = m.insert_after(&stage.last_step,ProgressBar::new_spinner());
  stage.last_step = pb.clone();
  pb.set_style(pending_style());
  pb.set_message("Read shim.");
  steps.push_back(pb.clone());

  // write final exe
  let pb = m.insert_after(&stage.last_step,ProgressBar::new_spinner());
  stage.last_step = pb.clone();
  pb.set_style(pending_style());
  pb.set_message("Write final executable.");
  steps.push_back(pb.clone());

  match BUILD_DATA.get() {
    Some(data) => {
      let pairs = {
        let mut data = data.lock().unwrap();
        vec![
          ("program".to_string(), data.bytecode.clone()),
        ]
      };

      // Create the zip
      let pb = steps.pop_front().unwrap();
      pb.set_style(build_style());
      pb.set_message("Compressing bytecode...");
      pb.enable_steady_tick(Duration::from_millis(100));
      let zip_bytes = match create_zip_from_pairs(&pairs) {
        Ok(b) => {
          pb.finish_with_message(format!("Created zip ({} bytes).", b.len()));
          b
        }
        Err(e) => {
          pb.set_style(fail_style());
          pb.finish_with_message(format!("Failed to create zip: {:?} {}", e, style("✗").red()));
          cancel_all("Build cancelled due to internal error.");
          stage.fail();
          return;
        }
      };

      // Find the built exe
      let pb = steps.pop_front().unwrap();
      pb.set_style(build_style());
      pb.set_message("Locating built shim executable...");
      pb.enable_steady_tick(Duration::from_millis(100));
      let build_project_dir = get_build_project_dir();

      let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
      let build_dir = current_dir.join(BUILD_DIR);

      let built_exe = match find_built_exe(&build_dir, &"mech_shim", None, release) {
        Ok(p) => {
          pb.finish_with_message(format!("Read built shim executable ({} bytes)", p.metadata().map(|m| m.len()).unwrap_or(0)));
          p
        }
        Err(e) => {
          pb.set_style(fail_style());
          pb.finish_with_message(format!("Failed to find built shim executable: {:?} {}", e, style("✗").red()));
          cancel_all("Build cancelled due to missing shim executable.");
          stage.fail();
          return;
        }
      };

      // Write the final exe
      let pb = steps.pop_front().unwrap();
      pb.set_style(build_style());
      pb.set_message("Writing final executable...");
      pb.enable_steady_tick(Duration::from_millis(100));
      let output_name = get_output_name().unwrap_or("mech_app".to_string());
      let out_path = Path::new(BUILD_DIR).join(format!("{}.exe",output_name));
      match write_final_exe(&built_exe, &zip_bytes, &out_path) {
        Ok(exe_size) => {
          pb.finish_with_message(format!("Wrote final executable ({} bytes)", exe_size));
          set_final_artifact_path(out_path.clone());
        }
        Err(e) => {
          pb.set_style(fail_style());
          pb.finish_with_message(format!("Failed to write final executable to {}: {} {}", out_path.canonicalize().unwrap_or_else(|_| out_path.clone()).display(), e, style("✗").red()));
          cancel_all("Build cancelled due to IO error.");
          stage.fail();
          return;
        }
      }
      build_progress.inc(1);
    }
    _ => {
      pb.set_style(fail_style());
      pb.finish_with_message(format!("No build data found. {}", style("✗").red()));
      cancel_all("Build cancelled due to internal error.");
      stage.fail();
      return;
    }
  }
  stage.build_progress.inc(1);
  stage.finish();
}

fn create_zip_from_pairs(pairs: &[(String, Vec<u8>)]) -> MResult<Vec<u8>> {
  let mut buf = Vec::new();
  {
    let mut zip = ZipWriter::new(Cursor::new(&mut buf));
    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, bytes) in pairs {
      let entry_name = format!("{}.mecb", name);
      match zip.start_file(&entry_name, options) {
        Ok(_) => {}
        Err(e) => {
          return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Failed to add file to zip: {}", e), id: line!(), kind: MechErrorKind::GenericError("Zip error".to_string())});
        }
      }
      match zip.write_all(bytes) {
        Ok(_) => {}
        Err(e) => {
          return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Failed to write file to zip: {}", e), id: line!(), kind: MechErrorKind::GenericError("Zip error".to_string())});
        }
      }
    }
    match zip.finish() {
      Ok(_) => {}
      Err(e) => {
        return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Failed to finish zip: {}", e), id: line!(), kind: MechErrorKind::GenericError("Zip error".to_string())});
      }
    }
  }
  Ok(buf)
}

fn find_built_exe(project_dir: &Path, shim_name: &str, target: Option<&str>, release: bool) -> MResult<PathBuf> {
  let mut candidate = project_dir.to_path_buf();
  if let Some(t) = target {
    candidate = candidate.join(t);
  }

  let mode = if release { "release" } else { "debug" };

  candidate = candidate.join(mode).join(shim_name);

  #[cfg(windows)]
  {
    let with_exe = candidate.with_extension("exe");
    if with_exe.exists() {
      return Ok(with_exe);
    } else {
      return Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Built executable not found at {}", with_exe.display()), id: line!(), kind: MechErrorKind::GenericError("Missing executable".to_string())});
    }
  }
  if candidate.exists() {
    return Ok(candidate);
  }
  Err(MechError {file: file!().to_string(), tokens: vec![], msg: format!("Built executable not found at {}", candidate.display()), id: line!(), kind: MechErrorKind::GenericError("Missing executable".to_string())})
}

fn write_final_exe(built_exe: &Path, zip_bytes: &[u8], out_path: &Path) -> Result<u64> {
  let mut exe_bytes = Vec::new();
  File::open(built_exe)?.read_to_end(&mut exe_bytes)?;
  let mut out = File::create(out_path)?;
  out.write_all(&exe_bytes)?;
  out.write_all(zip_bytes)?;
  let zip_size = zip_bytes.len() as u64;
  out.write_all(&zip_size.to_le_bytes())?;
  out.flush()?;
  // Return the total size of the output file
  Ok(exe_bytes.len() as u64 + zip_size + 8)
}