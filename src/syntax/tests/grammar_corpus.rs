use mech_syntax::parser;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

const CORPUS_MANIFEST: &str = include_str!("fixtures/grammar/corpus.tsv");
const CORPUS_ROOTS: [&str; 3] = ["docs", "examples", "mika"];
const RAW_PARSER_ENTRY_PATH: &str = "mech_syntax::parser::parse";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ExpectedOutcome {
    Accept,
    Reject,
    RequiresPreprocessing,
    Excluded,
}

impl ExpectedOutcome {
    fn from_manifest(value: &str) -> Option<Self> {
        match value {
            "accept" => Some(Self::Accept),
            "reject" => Some(Self::Reject),
            "requires-preprocessing" => Some(Self::RequiresPreprocessing),
            "excluded" => Some(Self::Excluded),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CorpusEntry<'a> {
    path: &'a str,
    outcome: ExpectedOutcome,
    entry_path: &'a str,
    reason: &'a str,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest_entries() -> Vec<CorpusEntry<'static>> {
    let mut lines = CORPUS_MANIFEST.lines();
    assert_eq!(
        lines.next(),
        Some("path\toutcome\tentry-path\treason"),
        "corpus.tsv must retain its documented four-column schema"
    );

    lines
        .enumerate()
        .filter(|(_, line)| !line.is_empty())
        .map(|(index, line)| {
            let line_number = index + 2;
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(
                fields.len(),
                4,
                "corpus.tsv line {line_number} must contain exactly four tab-separated fields"
            );

            let outcome = ExpectedOutcome::from_manifest(fields[1]).unwrap_or_else(|| {
                panic!(
                    "corpus.tsv line {line_number} has unsupported outcome {:?}",
                    fields[1]
                )
            });
            assert!(
                !fields[0].is_empty(),
                "corpus.tsv line {line_number} has an empty path"
            );
            assert!(
                !fields[2].is_empty(),
                "corpus.tsv line {line_number} has an empty entry path"
            );
            assert!(
                !fields[3].is_empty(),
                "corpus.tsv line {line_number} has an empty reason"
            );

            CorpusEntry {
                path: fields[0],
                outcome,
                entry_path: fields[2],
                reason: fields[3],
            }
        })
        .collect()
}

fn collect_mec_files(
    directory: &Path,
    repository_root: &Path,
    paths: &mut BTreeSet<String>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_mec_files(&path, repository_root, paths)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("mec") {
            let relative = path
                .strip_prefix(repository_root)
                .expect("corpus path must be beneath the repository root")
                .to_string_lossy()
                .replace('\\', "/");
            paths.insert(relative);
        }
    }
    Ok(())
}

fn repository_mec_files() -> BTreeSet<String> {
    let repository_root = repository_root();
    let mut paths = BTreeSet::new();
    for root in CORPUS_ROOTS {
        collect_mec_files(&repository_root.join(root), &repository_root, &mut paths)
            .unwrap_or_else(|error| panic!("failed to inventory {root}/: {error}"));
    }
    paths
}

#[test]
fn manifest_exactly_inventories_repository_mec_files() {
    let entries = manifest_entries();
    let mut manifest_paths = BTreeSet::new();
    let mut duplicates = BTreeSet::new();

    for entry in &entries {
        if !manifest_paths.insert(entry.path.to_owned()) {
            duplicates.insert(entry.path.to_owned());
        }
        assert!(
            CORPUS_ROOTS
                .iter()
                .any(|root| entry.path.starts_with(&format!("{root}/"))),
            "corpus entry is outside the inventoried roots: {}",
            entry.path
        );
        assert!(
            entry.path.ends_with(".mec"),
            "corpus entry is not a .mec file: {}",
            entry.path
        );
    }

    assert!(
        duplicates.is_empty(),
        "duplicate corpus entries: {duplicates:#?}"
    );

    let repository_paths = repository_mec_files();
    let missing = repository_paths
        .difference(&manifest_paths)
        .cloned()
        .collect::<Vec<_>>();
    let stale = manifest_paths
        .difference(&repository_paths)
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty() && stale.is_empty(),
        "corpus.tsv drifted from the repository\nmissing entries: {missing:#?}\nstale entries: {stale:#?}"
    );
}

#[test]
fn raw_parser_outcomes_match_manifest() {
    let repository_root = repository_root();
    let entries = manifest_entries();
    let mut failures = Vec::new();
    let mut outcome_counts = BTreeMap::new();

    for entry in entries {
        *outcome_counts.entry(entry.outcome).or_insert(0usize) += 1;

        match entry.outcome {
            ExpectedOutcome::Accept | ExpectedOutcome::Reject => {
                if entry.entry_path != RAW_PARSER_ENTRY_PATH {
                    failures.push(format!(
                        "{}: accept/reject entries must use {RAW_PARSER_ENTRY_PATH}, found {}",
                        entry.path, entry.entry_path
                    ));
                    continue;
                }

                let source = match fs::read_to_string(repository_root.join(entry.path)) {
                    Ok(source) => source,
                    Err(error) => {
                        failures.push(format!("{}: could not read fixture: {error}", entry.path));
                        continue;
                    }
                };
                let parsed =
                    panic::catch_unwind(AssertUnwindSafe(|| parser::parse(&source).is_ok()));
                match parsed {
                    Err(_) => failures.push(format!("{}: raw parser panicked", entry.path)),
                    Ok(actual_accepts) => {
                        let expected_accepts = entry.outcome == ExpectedOutcome::Accept;
                        if actual_accepts != expected_accepts {
                            failures.push(format!(
                                "{}: expected {}, raw parser returned {}",
                                entry.path,
                                if expected_accepts { "accept" } else { "reject" },
                                if actual_accepts { "accept" } else { "reject" }
                            ));
                        }
                    }
                }
            }
            ExpectedOutcome::RequiresPreprocessing | ExpectedOutcome::Excluded => {
                if entry.reason.trim().is_empty() {
                    failures.push(format!(
                        "{}: skipped corpus entries must document a reason",
                        entry.path
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "repository corpus outcomes changed (counts: {outcome_counts:?})\n{}",
        failures.join("\n")
    );
}
