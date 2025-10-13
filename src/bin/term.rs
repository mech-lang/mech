#![allow(warnings)]
use std::{
  thread,
  time::{Duration, Instant},
  collections::{VecDeque, HashMap},
  io::Write,
  sync::{mpsc, Arc, OnceLock, Mutex, atomic::{AtomicBool, Ordering}},
  env,
  path::{Path, PathBuf, MAIN_SEPARATOR},
  fs,
};
use console::{style, Emoji};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use rand::prelude::IndexedRandom;
use rand::Rng;
use clap::{arg, command, value_parser, Arg, ArgAction, Command};
use colored::Colorize;

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
}

fn init_cancel_flag() {
  CANCELLED.set(Arc::new(AtomicBool::new(false))).ok();
  ERROR_MESSAGE.set(Arc::new(Mutex::new(None))).ok();
  BUILD_DATA.set(Arc::new(Mutex::new(BuildData::default())));
}

fn set_bytecode(bytes: Vec<u8>) {
  if let Some(data) = BUILD_DATA.get() {
    let mut data = data.lock().unwrap();
    data.bytecode = bytes;
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
    final_state.finish_with_message("Artifact available at ./build/release/ekf.exe");
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

  pub fn add_build_stage(&mut self, mut stage: BuildStage) {
    match self.build_status {
      StepStatus::NotStarted => self.start(),
      _ => {}
    }
    
    // Apply Header To Section
    let header_style = ProgressStyle::with_template(
      "{prefix:.yellow.dim} {msg:.dim} {spinner:.dim}"
    ).unwrap()
     .tick_strings(&EMPTY);
    let header = self.indicators.insert_before(&self.build_progress, ProgressBar::new_spinner());
    header.set_style(header_style);
    header.set_prefix(format!("{}❱", stage.id));
    header.set_message(stage.name.clone());

    stage.header = header.clone(); 
    stage.last_step = header.clone();
    stage.build_progress = self.build_progress.clone();
    stage.indicators = Some(self.indicators.clone());
    // if it's the first stage we'll start it right away
    if self.stage_handles.is_empty() {
      let join_handle = thread::spawn(move || {
        stage.start();
      });
      self.stage_handles.push(join_handle);
    } else {
      stage.status = StepStatus::Pending;
      self.stages.push_back(stage);
    }
    self.build_progress.inc_length(1);   
  }

}

struct BuildStage {
  id: u64,
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
  pub fn new<F>(id: u64, name: impl Into<String>, f: F) -> Self
  where
      F: FnOnce(&mut BuildStage) + Send + 'static,
  {
    let style = ProgressStyle::with_template("{prefix:.yellow} {msg} {spinner:.dim}")
        .unwrap()
        .tick_strings(&DOTSPINNER);

    Self {
      id,
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
    .arg(Arg::new("tree")
        .short('t')
        .long("tree")
        .help("Print the syntax tree")
        .action(ArgAction::SetTrue))
    .subcommand(Command::new("build")
      .about("Build Mech program into a binary.")
      .arg(Arg::new("mech_build_file_paths")
        .help("Source .mec and .mecb files")
        .required(false)
        .action(ArgAction::Append))
      .arg(Arg::new("release")
        .short('d')
        .long("debug")
        .help("Print debug info")
        .action(ArgAction::SetTrue))        
      .arg(Arg::new("output_path")
        .short('o')
        .long("out")
        .help("Destination folder.")
        .required(false)))            
    .get_matches();

  let debug_flag = matches.get_flag("debug");
  let tree_flag = matches.get_flag("tree");
  let mech_paths: Vec<String> = matches.get_many::<String>("mech_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());

  if let Some(matches) = matches.subcommand_matches("build") {
    let build_paths: Vec<String> = matches.get_many::<String>("mech_build_file_paths").map_or(vec![], |files| files.map(|file| file.to_string()).collect());
    let release_flag = matches.get_flag("release");
    let output_path = matches.get_one::<String>("output_path").map(|s| s.to_string()).unwrap_or("./build".to_string());
    todo!("Build command not yet implemented");
  }

  if mech_paths.is_empty() {
    // use the current directory
    args.push(".".to_string());
    let current_dir = env::current_dir().unwrap();
    add_source(current_dir.to_str().unwrap());
    println!("No source files provided, using current directory: {}", current_dir.display());
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
    let mut prepare_build = BuildStage::new(1, "Prepare build environment", |mut stage| {
      prepare_build(&mut stage, path_tx2);
    });

    let mut download_packages = BuildStage::new(2, "Download packages", |mut stage| {
      download_packages(&mut stage,path_tx);
    });

    let mut build_packages = BuildStage::new(3, "Build packages", |mut stage| {
      parse_packages(&mut stage,path_rx, tree_tx);
    });

    let mut build_project = BuildStage::new(4, "Build project", |mut stage| {
      build_project(&mut stage, tree_rx);
    });

    let mut package_artifacts = BuildStage::new(5, "Package artifacts", |mut stage| {
      
    });

    let status = build.status_bar.clone();

    build.add_build_stage(prepare_build);
    build.add_build_stage(download_packages);
    build.add_build_stage(build_packages);
    build.add_build_stage(build_project);
    build.add_build_stage(package_artifacts);
    
    status.set_message("Preparing environment...");

    let stage_handles = std::mem::take(&mut build.stage_handles);
    for handle in stage_handles {
      let _ = handle.join();
    }

    // start the next stage:
    status.set_message("Downloading and building packages...");
    let mut download_stage = build.stages.pop_front().unwrap();
    let jh1 = thread::spawn(move || {
      download_stage.start();
    });

    let mut build_stage = build.stages.pop_front().unwrap();
    let jh2 = thread::spawn(move || {
      build_stage.start();
    });
    jh1.join();
    jh2.join();

    // Next stage
    status.set_message("Packaging...");
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
  step.finish_with_message(format!("Build directory ready {}", style("✓").green())); 

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
  step.finish_with_message(format!("Found {} source files. {}", source_files.len(), style("✓").green()));

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

  // Step 4
  let step = steps.pop_front().unwrap();
  step.set_style(build_style());
  step.finish_with_message(format!("Targeting Mech version {} {}", VERSION, style("✓").green()));

  // Step 5 - Configure environment
  let step = steps.pop_front().unwrap();
  step.set_style(build_style());
  step.finish_with_message(format!("Configured build environment {}", style("✓").green()));


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