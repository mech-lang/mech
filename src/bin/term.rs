use std::thread;
use std::time::{Duration, Instant};

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

#[derive(Debug)]
struct BuildProcess {
  id: u64,
  name: String,
  build_status: StepStatus,
  indicators: MultiProgress,
  sub_process_handles: Vec<thread::JoinHandle<()>>,
  start_time: Option<Instant>,
  end_time: Option<Instant>,
  stages: Vec<BuildStage>,
  build_progress: ProgressBar,
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

    BuildProcess {
      id, name,
      build_status: StepStatus::NotStarted,
      indicators: m,
      sub_process_handles: Vec::new(),
      start_time: None,
      end_time: None,
      stages: Vec::new(),
      build_progress,
    }
  }

  pub fn start(&mut self) {
    self.start_time = Some(Instant::now());
    self.build_status = StepStatus::Running;
  }

  pub fn finish(&mut self) {
    self.end_time = Some(Instant::now());
    self.build_status = StepStatus::Completed;
    self.build_progress.finish();
  }

  pub fn fail(&mut self) {
    self.end_time = Some(Instant::now());
    self.build_status = StepStatus::Failed;
    self.build_progress.finish();
  }

  pub fn add_build_stage(&mut self, mut stage: BuildStage) {
    match self.build_status {
      StepStatus::NotStarted => self.start(),
      _ => {}
    }
    
    // Apply Header To Section
    let header_style = ProgressStyle::with_template(
      "{prefix:.yellow} {msg} {spinner}"
    ).unwrap()
     .tick_strings(DOTSPINNER);
    let header = self.indicators.insert_before(&self.build_progress, ProgressBar::new_spinner());
    header.set_style(header_style);
    header.set_prefix(format!("{}❱", stage.id));
    header.set_message(stage.name.clone());
    header.enable_steady_tick(Duration::from_millis(100));

    stage.header = header.clone(); 
    stage.build_progress = self.build_progress.clone();
    stage.indicators = Some(self.indicators.clone());
    // if it's the first stage we'll start it right away
    if self.stages.is_empty() {
      stage.start();
    } else {
      stage.status = StepStatus::Pending;
    }
    self.stages.insert(self.stages.len(), stage);
    self.build_progress.inc_length(1);   

  }

}

#[derive(Debug)]
struct BuildStage {
  id: u64,
  name: String,
  status: StepStatus,
  start_time: Option<Instant>,
  end_time: Option<Instant>,
  steps: Vec<BuildStep>,
  header: ProgressBar,
  last: ProgressBar,
  stage_progress: ProgressBar,
  pub build_progress: ProgressBar,
  pub indicators: Option<MultiProgress>,
}

impl BuildStage {

  pub fn new(id: u64, name: String) -> Self {  
    BuildStage {
      id, name,
      status: StepStatus::NotStarted,
      start_time: None,
      end_time: None,
      steps: Vec::new(),
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
  }

  fn run_stage(&mut self, num_tasks: u32) {
    let m = self.indicators.clone().unwrap();

    let mut rng = rand::rng();
    let mut total_tasks = num_tasks;
    let mut handles = Vec::new();

    // initial tasks
    for _ in 0..num_tasks {
      handles.push(self.spawn_package_task(&m, &mut rng));
    }

    // dynamically discover new packages
    /*for _ in 0..3 {
      thread::sleep(Duration::from_millis(rng.random_range(1000..2000)));
      let new = rng.random_range(1..5);
      total_tasks += new;
      for _ in 0..new {
        handles.push(self.spawn_package_task(&m, &mut rng));
      }
    }*/

    for h in handles {
      let _ = h.join();
    }

    self.header.finish_with_message(format!("{} {}", self.name, style("✓").green()))
    //self.finish();
  }

  fn spawn_package_task(&mut self, m: &MultiProgress, rng: &mut rand::rngs::ThreadRng) -> thread::JoinHandle<()> {
    
    let build_progress = self.build_progress.clone();
    
    let spinner_style = ProgressStyle::with_template(
      "{prefix:.bold} {spinner:.yellow} {wide_msg}"
    ).unwrap()
    .tick_chars(SQUARESPINNER);
        
    let count = rng.random_range(30..80);
    let pb = m.insert_before(&self.build_progress, ProgressBar::new_spinner());
    pb.set_style(spinner_style.clone());
    pb.set_prefix("  ");
    thread::spawn(move || {
      let mut rng = rand::rng();
      let pkg = PACKAGES.choose(&mut rng).unwrap();
      build_progress.inc_length(1);
      for _ in 0..count {
        let cmd = COMMANDS.choose(&mut rng).unwrap();
        thread::sleep(Duration::from_millis(rng.random_range(25..200)));
        pb.set_message(format!("{pkg}: {cmd}"));
        pb.inc(1);
      }
      pb.set_message(format!("{pkg}: done"));
      pb.finish();
      build_progress.inc(1);
    })
  }

}

#[derive(Debug)]
struct BuildStep {
  id: u64,
  name: String,
  start_time: Option<Instant>,
  end_time: Option<Instant>,
  status: StepStatus,
  step_progress: ProgressBar,
  pub stage_progress: ProgressBar,
  pub build_progress: ProgressBar,
  pub indicators: Option<MultiProgress>,
}

impl BuildStep {

  pub fn new(id: u64, name: String) -> Self {
    BuildStep {
      id, name,
      start_time: None,
      end_time: None,
      status: StepStatus::NotStarted,
      step_progress: ProgressBar::new(0),
      stage_progress: ProgressBar::new(0),
      build_progress: ProgressBar::new(0),
      indicators: None,
    }
  }

  pub fn start(&mut self) {
    self.start_time = Some(Instant::now());
    self.status = StepStatus::Running;
  }

  pub fn finish(&mut self) {
    self.end_time = Some(Instant::now());
    self.status = StepStatus::Completed;
    self.step_progress.finish();
    self.stage_progress.inc(1);
  }

  pub fn fail(&mut self) {
    self.end_time = Some(Instant::now());
    self.status = StepStatus::Failed;
    self.step_progress.finish();
  }

}

pub fn main() {
  let mut build = BuildProcess::new(42, "Mech Builder".to_string());

  let mut prepare_environment = BuildStage::new(1, "Preparing build environment".to_string());
  //let mut download_packages = BuildStage::new(2, "Downloading packages".to_string());
  //let mut build_packages = BuildStage::new(3, "Building packages".to_string());

  build.add_build_stage(prepare_environment);
  //build.add_build_stage(download_packages);
  //build.add_build_stage(build_packages);



  /*
  let mut rng = rand::rng();
  let started = Instant::now();

  let progress_style = ProgressStyle::with_template(
    "{prefix:.dim} {bar:20.yellow/white.dim.bold} {percent}% ({pos}/{len})"
  ).unwrap()
   .progress_chars(PARALLELOGRAMPROGRESS);

  let m = MultiProgress::new();

  println!("{} Linking dependencies...", style("[Compiler]").yellow());
  let deps = 123;
  let pb = m.add(ProgressBar::new(deps));
  pb.set_style(progress_style.clone());
  pb.set_prefix("[Linking]");
  for _ in 0..deps {
    thread::sleep(Duration::from_millis(3));
    pb.inc(1);
  }
  pb.finish_and_clear();

  let m1 = m.clone();
  let m2 = m.clone();
  run_stage("1❱", 1, "Preparing build environment", &m);

  let deps = 1230;
  let pb = m.add(ProgressBar::new(deps));
  pb.set_style(progress_style.clone());
  pb.set_prefix("[Linking]");
  for _ in 0..deps {
    thread::sleep(Duration::from_millis(3));
    pb.inc(1);
  }
  pb.finish();

  // run stages 2 and 3 concurrently
  let handle1 = thread::spawn(move || {
      run_stage("2❱", 5, "Downloading packages", &m1);
  });

  let handle2 = thread::spawn(move || {
      run_stage("3❱", 3, "Building packages", &m2);
  });

  // wait for both concurrent stages to finish
  let _ = handle1.join();
  let _ = handle2.join();

  // Finalize build
  run_stage("4❱", 7, "Finalizing build", &m);

  let completed_style = ProgressStyle::with_template(
    "{prefix:.yellow} {msg} {spinner}"
  ).unwrap()
    .tick_strings(FISTBUMP);
  let completed = m.add(ProgressBar::new_spinner());
  completed.set_style(completed_style);

  // Run the fistbump animation
  for _ in 0..FISTBUMP.len() - 1 {
    thread::sleep(Duration::from_millis(100));
    completed.tick();
  }
  completed.finish();

  println!("Done in {}", HumanDuration(started.elapsed()));*/
}

