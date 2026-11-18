use mech_syntax::document::{
    DocumentId, DocumentStream, ParseConfig, StreamLimits, StreamProgress,
};
use std::path::Path;

fn check(text: &str, allowance: u64, limit: u64) -> serde_json::Value {
    let mut stream = DocumentStream::with_limits(
        DocumentId(3),
        ParseConfig::default(),
        StreamLimits { max_parser_work: limit, ..StreamLimits::default() },
    );
    stream.append(text, 0).unwrap();
    let mut update = stream.finish(allowance);
    while update.progress == StreamProgress::NeedsProcessing {
        update = stream.advance(allowance);
    }
    let finished = stream.materialize().ok().map(|snapshot| {
        serde_json::json!({
            "tree": snapshot.root.structural_hash,
            "diagnostics": format!("{:?}", snapshot.diagnostics),
            "stats": format!("{:?}", snapshot.stats),
        })
    });
    serde_json::json!({
        "allowance": allowance, "limit": limit, "bytes": text.len(),
        "progress": format!("{:?}", update.progress),
        "state": format!("{:?}", stream.state()),
        "work": format!("{:?}", stream.work()),
        "final": finished,
    })
}

fn run() {
    let path = std::env::args_os().nth(1).unwrap();
    let source = std::fs::read_to_string(Path::new(&path)).unwrap();
    let mut observations = vec![check(&source, u64::MAX, u64::MAX)];
    for (input, allowance, limit) in [
        ("x := 1\ny := x + 2\n", 1, u64::MAX),
        ("x := 1\ny := x + 2\n", 12, u64::MAX),
        ("x := 1\ny := x + 2\n", u64::MAX, u64::MAX),
        ("x := [1, 2,", 64, u64::MAX),
        ("x := \"a\\u{03b1}\"\n", 23, u64::MAX),
        ("x := [1, 2,", 9, 59),
        ("x := 1\ny := 2\n", 17, 125),
    ] {
        observations.push(check(input, allowance, limit));
    }
    println!("{}", serde_json::to_string_pretty(&observations).unwrap());
}
fn main() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024)
        .spawn(run).unwrap().join().unwrap();
}
