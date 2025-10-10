use std::thread;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::io::Write;
use console::{style, Emoji};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use rand::prelude::IndexedRandom;
use rand::Rng;

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

static SAND: &[&str] = &["⠁","⠂","⠄","⡀","⡈","⡐","⡠","⣀","⣁","⣂","⣄","⣌","⣔","⣤","⣥","⣦","⣮","⣶","⣷","⣿","⡿","⠿","⢟","⠟","⡛","⠛","⠫","⢋","⠋","⠍","⡉","⠉","⠑","⠡","⢁"," "];

static FISTBUMP: &[&str] = &[
  "   🤜　　　　🤛 ",
  "   🤜　　　　🤛 ",
  "   🤜　　　　🤛 ",
  "   　🤜　　🤛　 ",
  "   　　🤜🤛　　 ",
  "   　🤜💥🤛　　 ",
  "   🤜　✨　🤛　 ",
  "   ✌️　　　　✌️ "
];

static DOTSPINNER: &[&str] = &[
  "∙∙∙","∙∙∙","●∙∙","●∙∙","•●∙","•●∙","∙•●","∙•●","∙∙•","∙∙•","∙∙∙","∙∙∙"," "
];

static HEADASPLODE: &[&str] = &[
  "🙂","🙂","😐","😐","😮","😮","😦","😦","😧","😧","🤯","🤯","💥","✨","💭","　","　",
];

static SQUARESPINNER: &str = "◰◰◳◳◲◲◱◱▣";

static PARALLELOGRAMPROGRESS: &str = "▰▱";

// The Multiprogress thing is a bit messy to manage, so encapsulate it here
// along with the sub-process handles so they can be joined later
// There are three levels here. The entire Build may have multiple build processes
// Each build process can have many stages. Each stage can have many steps.
// Build process
// Build stage
// Build step
// We're going to handle this all with a single multiprogress instance for simplicity
// but we need to keep track of the hierarchy for reporting purposes
// and to manage the sub-process threads.

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
      "{prefix:.yellow} {bar:40.yellow/white.dim.bold} {percent}% ({pos}/{len})"
    ).unwrap()
     .progress_chars(PARALLELOGRAMPROGRESS);
    let m = MultiProgress::new();
    let build_progress = m.add(ProgressBar::new(0));
    build_progress.set_style(progress_style);
    build_progress.set_prefix("[Build]");

    // The status bar will be a spinner and it will also show the elapsed time
    let status_style = ProgressStyle::with_template(
      "{spinner:.yellow} {msg} [{elapsed_precise}]"
    ).unwrap()
     .tick_strings(SAND);
    let status_bar = m.add(ProgressBar::new_spinner());
    status_bar.set_style(status_style);
    status_bar.enable_steady_tick(Duration::from_millis(100));
    status_bar.set_message("Building...");

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
    self.build_status = StepStatus::  Running;
  }

  pub fn finish(&mut self) {
    self.end_time = Some(Instant::now());
    self.build_status = StepStatus::Completed;
    self.build_progress.finish();
    self.status_bar.finish_with_message(format!("Build completed in {}", HumanDuration(self.end_time.unwrap() - self.start_time.unwrap())));
  }

  pub fn fail(&mut self) {
    self.end_time = Some(Instant::now());
    self.build_status = StepStatus::Failed;
    self.build_progress.finish();
    self.status_bar.finish_with_message(format!("Build failed after {}", HumanDuration(self.end_time.unwrap() - self.start_time.unwrap())));
  }

  pub fn add_build_stage(&mut self, mut stage: BuildStage) {
    match self.build_status {
      StepStatus::NotStarted => self.start(),
      _ => {}
    }
    
    // Apply Header To Section
    let header_style = ProgressStyle::with_template(
      "{prefix:.yellow} {msg:.dim} {spinner:.dim}"
    ).unwrap()
     .tick_strings(&EMPTY);
    let header = self.indicators.insert_before(&self.build_progress, ProgressBar::new_spinner());
    header.set_style(header_style);
    header.set_prefix(format!("{}❱", stage.id));
    header.set_message(stage.name.clone());

    stage.header = header.clone(); 
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
  last: ProgressBar,
  style: ProgressStyle,
  stage_progress: ProgressBar,
  pub build_progress: ProgressBar,
  pub indicators: Option<MultiProgress>,
}

impl BuildStage {

  pub fn new(id: u64, name: String) -> Self {  
    let header_style = ProgressStyle::with_template(
      "{prefix:.yellow} {msg} {spinner:.dim}"
    ).unwrap()
     .tick_strings(&DOTSPINNER);
    BuildStage {
      id, name,
      status: StepStatus::NotStarted,
      start_time: None,
      end_time: None,
      //steps: Vec::new(),
      style: header_style,
      header: ProgressBar::new(0),
      last: ProgressBar::new(0),
      stage_progress: ProgressBar::new(0),
      build_progress: ProgressBar::new(0),
      indicators: None,
    }
  }

  pub fn start(&mut self) {
    self.start_time = Some(Instant::now());
    self.status = StepStatus::Running;
    self.header.set_style(self.style.clone());
    self.header.enable_steady_tick(Duration::from_millis(100));
    self.run_stage(3);
  }

  pub fn finish(&mut self) {
    self.end_time = Some(Instant::now());
    self.status = StepStatus::Completed;
    self.stage_progress.finish();
    self.build_progress.inc(1);
    //self.header.finish()
  }

  pub fn fail(&mut self) {
    self.end_time = Some(Instant::now());
    self.status = StepStatus::Failed;
    self.stage_progress.finish();
  }

  /*
  pub fn add_build_step(&mut self, mut step: BuildStep) {
    self.stage_progress.inc_length(1);
    step.build_progress = self.build_progress.clone();
    step.stage_progress = self.stage_progress.clone();
    step.indicators = self.indicators.clone();
    self.steps.push(step);
    match self.status {
      StepStatus::NotStarted => self.start(),
      _ => {}
    }
  }*/

  fn run_stage(&mut self, num_tasks: u32) {
    let m = self.indicators.clone().unwrap();

    let mut rng = rand::rng();
    let mut total_tasks = num_tasks;
    let mut handles = Vec::new();


    /*
      let random_name = PACKAGES.choose(&mut rng).unwrap();
      let mut step = BuildStep::new(0, random_name.to_string());
      let mut step = self.add_build_step(step);
      let handle = self.steps.last_mut().unwrap().run_step();
      handles.push(handle);
    */

    // initial tasks
    for _ in 0..num_tasks {
      handles.push(self.spawn_package_task(&m, &mut rng));
    }

    // dynamically discover new packages
    for _ in 0..3 {
      thread::sleep(Duration::from_millis(rng.random_range(1000..2000)));
      let new = rng.random_range(1..5);
      total_tasks += new;
      for _ in 0..new {
        handles.push(self.spawn_package_task(&m, &mut rng));
      }
    }

    for h in handles {
      let _ = h.join();
    }

    self.header.finish_with_message(format!("{} {}", self.name, style("✓").green()))
  }

  fn spawn_package_task(&mut self, m: &MultiProgress, rng: &mut rand::rngs::ThreadRng) -> thread::JoinHandle<()> {
    
    let build_progress = self.build_progress.clone();
    
    let spinner_style = ProgressStyle::with_template(
      "{prefix:.bold} {spinner:.yellow} {wide_msg}"
    ).unwrap()
    .tick_chars(SQUARESPINNER);
        
    let count = rng.random_range(30..80);
    let pb = m.insert_after(&self.header, ProgressBar::new_spinner());
    pb.set_style(spinner_style.clone());
    pb.set_prefix("  ");
    thread::spawn(move || {
      let mut rng = rand::rng();
      let pkg = PACKAGES.choose(&mut rng).unwrap();
      build_progress.inc_length(1);
      for _ in 0..count {
        let cmd = COMMANDS.choose(&mut rng).unwrap();
        thread::sleep(Duration::from_millis(rng.random_range(25..50)));
        pb.set_message(format!("{pkg}: {cmd}"));
        pb.inc(1);
      }
      pb.set_message(format!("{pkg}: done"));
      pb.finish();
      build_progress.inc(1);
    })
  }

}

pub fn main() {
  let mut build = BuildProcess::new(42, "Mech Builder".to_string());
  let m = build.indicators.clone();

  let mut prepare_environment = BuildStage::new(1, "Preparing build environment".to_string());
  let mut download_packages = BuildStage::new(2, "Downloading packages".to_string());
  let mut build_packages = BuildStage::new(3, "Building packages".to_string());
  let mut linking = BuildStage::new(4, "Linking".to_string());

  let status = build.status_bar.clone();

  build.add_build_stage(prepare_environment);
  build.add_build_stage(download_packages);
  build.add_build_stage(build_packages);
  build.add_build_stage(linking);

  for handle in build.stage_handles {
    let _ = handle.join();
  }

  // start the next stage:
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

  let mut linking_stage = build.stages.pop_front().unwrap();
  linking_stage.start();

  status.finish_with_message("All done!");

  let progress = m.add(ProgressBar::new_spinner());
  let completed_style = ProgressStyle::with_template(
    "{prefix:.yellow} {msg} {spinner}"
  ).unwrap()
    .tick_strings(FISTBUMP);
  progress.set_style(completed_style);
  progress.set_prefix("[Done]");
  // Run the fistbump animation
  for _ in 0..FISTBUMP.len() - 1 {
    thread::sleep(Duration::from_millis(100));
    progress.tick();
  }
  progress.finish();
}

