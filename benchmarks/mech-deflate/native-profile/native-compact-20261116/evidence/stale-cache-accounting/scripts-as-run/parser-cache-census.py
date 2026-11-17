"""Trace cache publication, use and pending ranges in the current one-shot parser."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import shutil
import subprocess

q = Path(__file__).resolve().parent
checkout = q / "native-review-pr/checkout"
prior = q / "equivalent-vector-phases/parser-stages"
study = q / "parser-cache-liveness"
study.mkdir()
source = checkout / "benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec"
baseline = json.loads((prior / "report.json").read_text())
expected = baseline["trials"][0]
syntax = study / "syntax"
shutil.copytree(checkout / "src/syntax/src", syntax)
parser = syntax / "document/parser/mod.rs"
cache = syntax / "document/parser/tree_cache.rs"


def change(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new, 1)


text = cache.read_text()
text = change(text, "                        self.work.reused += 1;",
              "                        trace_reuse(cached.node.id);\n                        self.work.reused += 1;")
text = change(text, "        self.queue.push_back(Request {", "        trace_count(|c| c.enqueued += 1);\n        self.queue.push_back(Request {")
text = change(text, "            let (request, sink) = self.active.take().expect(\"completed cache request\");",
              "            let (request, sink) = self.active.take().expect(\"completed cache request\");\n            let mut published = false;")
text = change(text, "                    *slot = Some(cached);", "                    *slot = Some(cached);\n                    published = true;")
text = change(text, "            self.work.events += work.events;", "            trace_publication(expected_identity, published);\n            self.work.events += work.events;")
text = change(text, '            {\n                return false;\n            }\n            let cached = CachedNode {',
              '            {\n                trace_count(|c| c.partial_advances += 1);\n                return false;\n            }\n            let cached = CachedNode {')
text = change(text, "                let request = self.queue.pop_front().expect(\"pending cache request\");",
              '''                let request = self.queue.pop_front().expect("pending cache request");
                let live = matches!(current.get(request.start), Some(Event::Start { identity: Some(id), .. }) if *id == request.identity);
                trace_count(|c| c.stale_on_start += usize::from(!live));''')
text += '''
#[derive(Default)]
struct CacheCensus {
    enqueued: usize,
    published: usize,
    discarded: usize,
    stale_on_start: usize,
    partial_advances: usize,
    overlapping_structural_edits: usize,
    overlapping_rewinds: usize,
    overlapping_clears: usize,
    cached_reads: usize,
    identities: Vec<u8>,
}

std::thread_local! {
    static CENSUS: std::cell::RefCell<CacheCensus> = std::cell::RefCell::new(CacheCensus::default());
}

fn trace_count(f: impl FnOnce(&mut CacheCensus)) {
    CENSUS.with(|profile| f(&mut profile.borrow_mut()));
}

impl CacheCensus {
    fn mark(&mut self, id: NodeId, mask: u8) {
        let id = id.0 as usize;
        if id >= self.identities.len() {
            self.identities.resize(id + 1, 0);
        }
        self.identities[id] |= mask;
    }
}

fn trace_publication(identity: NodeId, published: bool) {
    trace_count(|census| {
        if published {
            census.published += 1;
            census.mark(identity, 1);
        } else {
            census.discarded += 1;
        }
    });
}

pub(super) fn trace_reuse(identity: NodeId) {
    trace_count(|census| {
        census.cached_reads += 1;
        census.mark(identity, 2);
    });
}

impl TreeCache {
    pub(super) fn trace_edit(&self, start: usize, end: usize, kind: u8) {
        let overlaps = self.queue.iter().chain(self.active.iter().map(|(request, _)| request)).any(
            |request| start < request.end && request.start < end,
        );
        if overlaps {
            trace_count(|census| match kind {
                0 => census.overlapping_structural_edits += 1,
                1 => census.overlapping_rewinds += 1,
                _ => census.overlapping_clears += 1,
            });
        }
    }
}

pub(super) fn trace_snapshot(snapshot: &SyntaxSnapshot) {
    trace_count(|census| {
        for (id, _) in snapshot.nodes.nodes() {
            census.mark(id, 4);
        }
        let masks = census.identities.iter().copied().fold([0usize; 8], |mut counts, mask| {
            counts[mask as usize] += 1;
            counts
        });
        eprintln!("cache-census {} {} {} {} {} {} {} {} {} {:?}",
            census.enqueued, census.published, census.discarded, census.cached_reads,
            census.stale_on_start, census.partial_advances, census.overlapping_structural_edits,
            census.overlapping_rewinds, census.overlapping_clears, masks);
    });
}
'''
cache.write_text(text)

text = parser.read_text()
anchors = (
    ("        let identity = self.ids.node();", "        self.state.tree_cache.trace_edit(marker.position, marker.position + 1, 0);"),
    ("        self.state.open_markers.pop();\n        if marker.position + 1 == self.state.events.len() {",
     "        self.state.tree_cache.trace_edit(marker.position, marker.position + 1, 0);"),
    ("                self.state.events[position] = Event::Tombstone;",
     "                self.state.tree_cache.trace_edit(position, position + 1, 0);"),
    ("        self.state.events.truncate(checkpoint.events);",
     "        self.state.tree_cache.trace_edit(checkpoint.events, self.state.events.len(), 1);"),
    ("            self.state.events.clear();", "            self.state.tree_cache.trace_edit(0, self.state.events.len(), 2);"),
    ("        self.state.cached_replay_bytes += u64::from((subtree.end - subtree.start).0);",
     "        tree_cache::trace_reuse(subtree.node.id);"),
    ("    snapshot.restarts = build_restart_index(&snapshot);", "    tree_cache::trace_snapshot(&snapshot);"),
)
for anchor, instrument in anchors:
    text = change(text, anchor, instrument + "\n" + anchor)
text = change(text, "        let state = self.suspend();\n        ParserOutput {", '''        eprintln!("event-journal-census {} {} {} {} {:?}", self.state.events.allocations,
            self.state.events.mutations, self.state.events.discarded, self.state.tree_cache.peak_pending,
            self.state.tree_cache.total_work());
        let state = self.suspend();
        ParserOutput {''')
parser.write_text(text)


def run(command, log):
    env = os.environ | {"CARGO_MANIFEST_DIR": str(checkout / "src/syntax"), "CARGO_PKG_VERSION": "0.3.5"}
    with (study / log).open("wb") as out:
        subprocess.run(command, env=env, stdout=out, stderr=subprocess.STDOUT, check=True)


lib = study / "libmech_syntax_census.rlib"
build = [str(syntax / "lib.rs") if arg.endswith("/syntax/lib.rs")
         else str(lib) if arg.endswith("libmech_syntax_profile.rlib")
         else "metadata=parser-cache-census" if arg == "metadata=parser-stages"
         else arg for arg in baseline["build_command"]]
run(build, "build.log")
probe = study / "probe"
link = [f"mech_syntax={lib}" if arg.startswith("mech_syntax=")
        else str(probe) if arg == str(prior / "instrumented") else arg
        for arg in baseline["link_commands"]["instrumented"]]
run(link, "link.log")
observations = []
for trial in range(1, 3):
    name = f"trial-{trial}"
    output = study / f"{name}.json"
    cmd = list(map(str, (probe, source, output)))
    run(cmd, name + ".log")
    result = json.loads(output.read_text())
    for field in ("tree_sha256", "parser_steps", "events", "source_sha256"):
        assert result[field] == expected[field], (trial, field)
    lines = (study / (name + ".log")).read_text().splitlines()
    info = next(line for line in lines if line.startswith("cache-census "))
    prefix, _, values = info.partition("[")
    keys = ("enqueued", "published", "discarded", "cached_reads", "stale_at_materialization_start",
            "partial_cache_advances", "overlapping_structural_edits", "overlapping_rewinds", "overlapping_clears")
    counts = dict(zip(keys, map(int, prefix.split()[1:]), strict=True))
    masks = json.loads("[" + values)
    counts.update(published_and_read=sum(masks[i] for i in (3, 7)), published_and_retained=sum(masks[i] for i in (5, 7)),
                  published_and_unread=masks[1] + masks[5], distinct_retained_nodes=sum(masks[4:]))
    journal = next(line for line in lines if line.startswith("event-journal-census "))
    observations.append({"command": cmd, "tree": result, "counts": counts, "raw_journal": journal})
assert observations[0]["counts"] == observations[1]["counts"]
with lib.open("rb") as file:
    digest = file_digest(file, "sha256").hexdigest()
report = {"scope": "Cache lifetime census. Instrumentation timings do not represent production performance.",
          "build": build, "link": link, "instrumented_syntax_sha256": digest, "trials": observations}
(study / "report.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(observations[0]["counts"], indent=2))
print(observations[0]["raw_journal"])
