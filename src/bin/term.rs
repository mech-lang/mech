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
  "🙂","🙂","😐","😐","😮","😮","😦","😦","😧","😧","🤯","🤯","💥","✨","　","　",
];

static SQUARESPINNER: &str = "◰◰◳◳◲◲◱◱▣";

fn run_stage(prefix: &'static str,num_tasks: u32,msg: &'static str, m: &MultiProgress) {
  let spinner_style = ProgressStyle::with_template(
    "{prefix:.bold} {spinner:.yellow} {wide_msg}"
  ).unwrap()
   .tick_chars(SQUARESPINNER);

  let header_style = ProgressStyle::with_template(
    "{prefix:.yellow} {msg} {spinner}"
  ).unwrap()
    .tick_strings(DOTSPINNER);
  
  let header = m.add(ProgressBar::new_spinner());
  header.set_style(header_style);
  header.set_prefix(prefix);
  header.set_message(msg);
  header.enable_steady_tick(Duration::from_millis(100));

  let mut rng = rand::rng();
  let mut total_tasks = num_tasks;
  let mut handles = Vec::new();

  // initial tasks
  for _ in 0..num_tasks {
    handles.push(spawn_package_task(m, &spinner_style, &mut rng));
  }

  // dynamically discover new packages
  for _ in 0..3 {
    thread::sleep(Duration::from_millis(rng.random_range(1000..2000)));
    let new = rng.random_range(1..5);
    total_tasks += new;
    for _ in 0..new {
      handles.push(spawn_package_task(m, &spinner_style, &mut rng));
    }
  }

  for h in handles {
    let _ = h.join();
  }

  header.finish_with_message(format!("{msg} {}", style("✓").green()));

}

fn spawn_package_task(m: &MultiProgress,spinner_style: &ProgressStyle,rng: &mut rand::rngs::ThreadRng) -> thread::JoinHandle<()> {
  let count = rng.random_range(30..80);
  let pb = m.add(ProgressBar::new(count));
  pb.set_style(spinner_style.clone());
  pb.set_prefix("  ");
  thread::spawn(move || {
    let mut rng = rand::rng();
    let pkg = PACKAGES.choose(&mut rng).unwrap();
    for _ in 0..count {
      let cmd = COMMANDS.choose(&mut rng).unwrap();
      thread::sleep(Duration::from_millis(rng.random_range(25..200)));
      pb.set_message(format!("{pkg}: {cmd}"));
      pb.inc(1);
    }
    pb.finish_and_clear();
  })
}

pub fn main() {
  let mut rng = rand::rng();
  let started = Instant::now();

  let progress_style = ProgressStyle::with_template(
    "{prefix:.dim} {bar:20.yellow/white.dim.bold} {percent}% ({pos}/{len})"
  ).unwrap()
   .progress_chars("▰▱");

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
  run_stage("1❱", 4, "Preparing build environment", &m);

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

  println!("Done in {}", HumanDuration(started.elapsed()));
}