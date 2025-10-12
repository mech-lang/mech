use std::thread;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::io::Write;
use console::{style, Emoji};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use rand::prelude::IndexedRandom;
use rand::Rng;
use std::sync::{mpsc, Arc, OnceLock, Mutex, atomic::{AtomicBool, Ordering}};

static ERROR_MESSAGE: OnceLock<Arc<Mutex<Option<String>>>> = OnceLock::new();

static CANCELLED: OnceLock<Arc<AtomicBool>> = OnceLock::new();

fn init_cancel_flag() {
  CANCELLED.set(Arc::new(AtomicBool::new(false))).ok();
  ERROR_MESSAGE.set(Arc::new(Mutex::new(None))).ok();
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

static PREPARE: &[&str] = &[
  "Check for required tools",
  "Create temp build directory",
  "Download base packages",
  "Set up environment variables",
];

static PACKAGES: &[&str] = &[
  "fs-events",
  "my-awesome-module",
  "emoji-speaker",
  "wrap-ansi",
  "stream-browserify",
  "acorn-dynamic-import",
];

static COMMANDS: &[&str] = &[
  "cmake .",
  "make",
  "make clean",
  "gcc foo.c -o foo",
  "gcc bar.c -o bar",
  "./helper.sh rebuild-cache",
  "make all-clean",
  "make test",
];

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
    for _ in 0..FISTBUMP.len() - 1 {
      thread::sleep(Duration::from_millis(100));
      final_state.tick();
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
    self.header.set_style(self.style.clone());
    self.header.enable_steady_tick(Duration::from_millis(100));
    self.task_fn.take().map(|f| f(self));
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
  init_cancel_flag();
  let start_time = Instant::now();
  println!(r#"{} Building: cmontella/ekf v0.2.3 (C:\cmont\Desktop\ekf)"#, style("[mech v0.2.60]").yellow());
  {
    let mut build = BuildProcess::new(42, "Mech Builder".to_string());
    let m = build.indicators.clone();
    let cancelled = Arc::new(AtomicBool::new(false));

    let mut prepare_environment = BuildStage::new(1, "Prepare build environment", |mut stage| {
      prepare_environment(&mut stage);
    });

    let (tx, rx) = mpsc::channel();

    let mut download_packages = BuildStage::new(2, "Download packages", |mut stage| {
      download_packages(&mut stage,tx);
    });

    let mut build_packages = BuildStage::new(3, "Build", |mut stage| {
      build_packages(&mut stage,rx);
    });

    let mut packaging = BuildStage::new(4, "Pack", |mut stage| {
        run_stage(&mut stage, 2);
    });

    let status = build.status_bar.clone();

    build.add_build_stage(prepare_environment);
    build.add_build_stage(download_packages);
    build.add_build_stage(build_packages);
    build.add_build_stage(packaging);
    
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

// SIMULATE THE BUILD PROCESS -------------------------------------------------

fn run_stage(stage: &mut BuildStage, num_tasks: u32) {
  if is_cancelled() {
    stage.cancel();
    return;
  }


  let build_progress = stage.build_progress.clone();
  let mut rng = rand::rng();
  let mut total_tasks = num_tasks;
  let mut handles = Vec::new();

  // initial tasks
  for _ in 0..num_tasks {
    handles.push(spawn_package_task(&stage));
  }

  // dynamically discover new packages
  for _ in 0..3 {
    thread::sleep(Duration::from_millis(rng.random_range(1000..2000)));
    let new = rng.random_range(1..5);
    total_tasks += new;
    for _ in 0..new {
      handles.push(spawn_package_task(&stage));
    }
  }

  for h in handles {
    let _ = h.join();
  }
  stage.finish();
}

fn spawn_package_task(stage: &BuildStage) -> thread::JoinHandle<()> {
  let m = stage.indicators.as_ref().unwrap().clone();
  let build_progress = stage.build_progress.clone();
  
  let spinner_style = ProgressStyle::with_template(
    "{prefix:.bold} {spinner:.yellow} {wide_msg}"
  ).unwrap()
  .tick_chars(SQUARESPINNER);
          
  let pb = m.insert_after(&stage.header, ProgressBar::new_spinner());
  pb.set_style(spinner_style.clone());
  pb.enable_steady_tick(Duration::from_millis(100));
  pb.set_prefix("  ");
  thread::spawn(move || {
    let mut rng = rand::rng();
    let pkg = PACKAGES.choose(&mut rng).unwrap();
    pb.set_message(format!("{pkg}"));
    thread::sleep(Duration::from_millis(rng.random_range(25..5000)));
    build_progress.inc_length(1);
    pb.set_message(format!("{pkg}: done"));
    pb.finish_and_clear();
    build_progress.inc(1);
  })
}

fn download_packages(stage: &mut BuildStage, tx: mpsc::Sender<String>) {
  if is_cancelled() {
    stage.cancel();
    return;
  }
  let m = stage.indicators.as_ref().unwrap().clone();
  let build_progress = stage.build_progress.clone();
  
  let download_style = ProgressStyle::with_template(
    "{prefix:.yellow} {spinner:.yellow} {msg} {bar:20.yellow/white.dim.bold} {percent}% ({pos}/{len})"
  ).unwrap()
   .progress_chars(PARALLELOGRAMPROGRESS)
   .tick_chars(SQUARESPINNER);

  let fail_style = ProgressStyle::with_template(
    "{prefix} {spinner:.red} {msg}",
  )
  .unwrap()
  .tick_chars(FAILEDSQUARESPINNER);

  let mut handles = Vec::new();
  for pkg in PACKAGES {
    // Random size per package
    let rand_size: u64 = rand::thread_rng().gen_range(50..100);
    build_progress.inc_length(1);
    let pb = m.insert_after(&stage.header, ProgressBar::new(rand_size));
    pb.set_style(download_style.clone());
    pb.set_prefix("  ");
    pb.set_message(format!("{:<20}", pkg));

    let tx = tx.clone();
    let failstyle = fail_style.clone();

    let build_progress = stage.build_progress.clone();
    let handle = thread::spawn(move || {
      for j in 0..=rand_size {
        if is_cancelled() {
          pb.set_style(failstyle.clone());
          pb.finish();
          return;
        }
        pb.set_position(j);
        thread::sleep(Duration::from_millis(20 + rand::thread_rng().gen_range(0..300)));
        // with 5% probability, fail the download
        if rand::thread_rng().gen_range(0..5000) < 1 && j > 20 && j < rand_size - 20 {
          pb.set_style(failstyle.clone());
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
  drop(tx);
  if is_cancelled() {
    stage.fail();
  } else {
    stage.finish();
  }
}

fn build_packages(stage: &mut BuildStage, rx: mpsc::Receiver<String>) {
  if is_cancelled() {
    stage.cancel();
    return;
  }
  let m = stage.indicators.as_ref().unwrap().clone();
  let build_progress = stage.build_progress.clone();

  let build_style = ProgressStyle::with_template(
      "{prefix} {spinner:.yellow} {msg}",
  ).unwrap()
   .tick_chars(SQUARESPINNER);

  let fail_style = ProgressStyle::with_template(
    "{prefix} {spinner:.red} {msg}",
  ).unwrap()
   .tick_chars(FAILEDSQUARESPINNER);

  let mut handles = Vec::new();

  // Read from the channel and spawn build tasks until the channel is closed
  for pkg in rx {
    build_progress.inc_length(1);
    let pb = m.insert_after(&stage.header, ProgressBar::new(0));
    pb.set_style(build_style.clone());
    pb.set_prefix("  ");
    pb.set_message(format!("{:<20}", pkg));
    pb.enable_steady_tick(Duration::from_millis(100));

    let build_progress = build_progress.clone();

    let failstyle = fail_style.clone();
    let handle = thread::spawn(move || {
      for j in 0..=100 {
        if is_cancelled() {
          pb.set_style(failstyle.clone());
          pb.finish();
          return;
        }
        pb.set_position(j);
        thread::sleep(Duration::from_millis(30 + rand::thread_rng().gen_range(0..150)));
        // probability 10% to fail
        if rand::thread_rng().gen_range(0..5000) < 0 && j > 20 && j < 90 {
          pb.set_style(failstyle.clone());
          pb.finish_with_message(format!("{:<20} {}", pkg, style("✗").red()));
          cancel_all(format!("Failed to build package: {}", pkg).as_str());
          return;
        }
      }
      pb.finish_and_clear();
      build_progress.inc(1);
    });
    handles.push(handle);
  }

  for handle in handles {
    let _ = handle.join();
  }

  if is_cancelled() {
    stage.fail();
  } else {
    stage.finish();
  }

}

fn prepare_environment(stage: &mut BuildStage) {
  if is_cancelled() {
    stage.cancel();
    return;
  }
  let m = stage.indicators.as_ref().unwrap().clone();
  let build_progress = stage.build_progress.clone();
  let download_style = ProgressStyle::with_template(
    "{prefix:.yellow} {spinner:.yellow} {msg} {bar:20.yellow/white.dim.bold} {percent}%"
  ).unwrap()
   .progress_chars(PARALLELOGRAMPROGRESS)
   .tick_chars(SQUARESPINNER);
  let pending_download = ProgressStyle::with_template(
    "{prefix:.yellow} {spinner:.yellow} {msg} {bar:20.yellow/white.dim.bold} {percent}%"
  ).unwrap()
   .progress_chars(PARALLELOGRAMPROGRESS)
   .tick_chars(PENDINGSQUARESPINNER);
  let mut pbs = Vec::new();
  for msg in PREPARE {
    build_progress.inc_length(1);
    let pb = m.insert_after(&stage.last_step, ProgressBar::new(0));
    stage.last_step = pb.clone();
    pb.set_style(pending_download.clone());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_prefix("  ");
    pb.set_message(format!("{:<30}", msg));
    pb.set_length(100);
    pbs.push(pb);
  }
  
  let fail_style = ProgressStyle::with_template(
    "{prefix} {spinner:.red} {msg}",
  ).unwrap()
   .tick_chars(FAILEDSQUARESPINNER);
  let failstyle = fail_style.clone();
  for pb in pbs.iter() {
    pb.set_style(download_style.clone());
    for j in 0..=100 {
      if is_cancelled() {
        pb.set_style(fail_style.clone());
        pb.finish();
        continue;
      }
      pb.set_position(j);
      thread::sleep(Duration::from_millis(20 + rand::thread_rng().gen_range(0..10)));
      // with 2% probability, fail the step
      if rand::thread_rng().gen_range(0..5000) < 1 && j > 20 && j < 90 {
        pb.set_style(failstyle.clone());
        pb.finish_with_message(format!("{:<30} {}", pb.message(), style("✗").red()));
        cancel_all(format!("Failed to prepare: {}", pb.message()).as_str());
        continue;
      }
    }
    pb.finish();
    build_progress.inc(1);
  }

  if is_cancelled() {
    stage.fail();
  } else {
    stage.finish();
  }
}
