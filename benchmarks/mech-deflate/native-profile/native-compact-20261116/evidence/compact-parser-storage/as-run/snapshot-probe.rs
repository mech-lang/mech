use mech_syntax::document::{DocumentId, ParseConfig, Revision, TextSnapshot, parse_canonical_document};
use sha2::{Digest, Sha256};
use std::fmt::Write;

fn digest(value: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn main() {
    for file in std::env::args().skip(1) {
        let text = std::fs::read_to_string(&file).unwrap();
        for split in [false, true] {
            let source = if split {
                let mut source = TextSnapshot::new(DocumentId(17), Revision(1), "").unwrap();
                for scalar in text.chars() {
                    source = source.append(scalar.to_string()).unwrap();
                }
                source
            } else {
                TextSnapshot::new(DocumentId(17), Revision(1), text.as_str()).unwrap()
            };
            let ordinary = ParseConfig::default();
            let mut limited_fuel = ordinary;
            limited_fuel.limits.fuel = 128;
            let mut limited_events = ordinary;
            limited_events.limits.max_events = 32;
            for (limit, config) in [("ordinary", ordinary), ("fuel", limited_fuel), ("events", limited_events)] {
                let parsed = parse_canonical_document(source.clone(), config);
                let mut result = String::new();
                write!(&mut result, "{:?}{:?}{:?}{:?}{:?}",
                       parsed.root, parsed.nodes, parsed.restarts, parsed.diagnostics, parsed.stats).unwrap();
                println!(
                    "{} {}",
                    digest(text.as_bytes()),
                    digest(format!("{split}-{limit}-{result}").as_bytes())
                );
            }
        }
    }
}
