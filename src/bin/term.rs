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

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static TRUCK: Emoji<'_, '_> = Emoji("🚚  ", "");
static CLIP: Emoji<'_, '_> = Emoji("🔗  ", "");
static PAPER: Emoji<'_, '_> = Emoji("📃  ", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");

fn run_stage(
    prefix: &str,
    emoji: Emoji,
    num_tasks: u32,
    spinner_style: &ProgressStyle,
    m: &MultiProgress,
) {
    println!("{} {}{}", style(prefix).bold().dim(), emoji, "Building fresh packages...");

    let mut rng = rand::rng();
    let mut total_tasks = num_tasks;
    let mut handles = Vec::new();

    // initial tasks
    for _ in 0..num_tasks {
        handles.push(spawn_package_task(m, spinner_style, &mut rng));
    }

    // dynamically discover new packages
    for _ in 0..3 {
        thread::sleep(Duration::from_millis(rng.random_range(1000..2000)));
        let new = rng.random_range(1..5);
        total_tasks += new;
        for _ in 0..new {
            handles.push(spawn_package_task(m, spinner_style, &mut rng));
        }
    }

    for h in handles {
        let _ = h.join();
    }
}

fn spawn_package_task(
    m: &MultiProgress,
    spinner_style: &ProgressStyle,
    rng: &mut rand::rngs::ThreadRng,
) -> thread::JoinHandle<()> {
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

    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner:.yellow} {wide_msg}")
        .unwrap()
        .tick_chars("◰◳◲◱▣");

    let progress_style = ProgressStyle::with_template(
        "{prefix:.dim} {bar:20.yellow/white.dim.bold} {percent}% ({pos}/{len})"
    )
    .unwrap()
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

    // run all stages using same MultiProgress
    run_stage("[1/4]", LOOKING_GLASS, 4, &spinner_style, &m);
    run_stage("[2/4]", TRUCK, 5, &spinner_style, &m);
    run_stage("[3/4]", CLIP, 3, &spinner_style, &m);
    run_stage("[4/4]", PAPER, 7, &spinner_style, &m);

    println!("{} Done in {}", SPARKLE, HumanDuration(started.elapsed()));
}
