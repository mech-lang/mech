"""Count requested event-window sizes without changing parsing or grants."""

import json
import shutil

from study_tools import build, checkout, link, probe, replace, sha, study, write

out = study / "census"
out.mkdir()
syntax = out / "source"
shutil.copytree(checkout / "src/syntax/src", syntax)
cache = syntax / "document/parser/tree_cache.rs"
parser = syntax / "document/parser/mod.rs"
text = cache.read_text()
text = replace(text, "        self.queue.push_back(Request {", """        let used = events.len() - start;
        let page = events.page_at(start).unwrap();
        LENGTHS.with(|items| {
            let mut items = items.borrow_mut();
            items.0[used.min(65)] += 1;
            items.1 += u64::from(events.len() <= page.0 + page.1.len());
        });
        self.queue.push_back(Request {""")
text += """
std::thread_local! {
    static LENGTHS: std::cell::RefCell<([u64; 66], u64)> =
        const { std::cell::RefCell::new(([0; 66], 0)) };
}

pub(super) fn print_window_lengths(peak_pending: usize, event_allocations: u64) {
    LENGTHS.with(|items| {
        let items = items.borrow();
        eprintln!("cache-window-lengths|{:?}|{}|{peak_pending}|{event_allocations}", items.0, items.1);
    });
}
"""
cache.write_text(text)
text = parser.read_text()
text = replace(text, "        let state = self.suspend();\n        ParserOutput {",
               """        tree_cache::print_window_lengths(self.state.tree_cache.peak_pending, self.state.events.allocations);
        let state = self.suspend();
        ParserOutput {""")
parser.write_text(text)
library, rustc = build(out, syntax, "queue-snapshot-census")
exe, link_cmd = link(out, library)
trial = probe(exe, out / "trial.json")
marker = next(line for line in (out / "trial.log").read_text().splitlines() if line.startswith("cache-window-lengths|"))
_, raw, page, pending, allocations = marker.split("|")
buckets = json.loads(raw)
result = {"scope":"Instrumented window census", "build": rustc, "link": link_cmd, "probe": trial,
          "library_sha256": sha(library), "requests": sum(buckets), "lengths_0_through_64_then_larger": buckets,
          "same_page": int(page), "peak_queue": int(pending), "journal_allocations": int(allocations)}
write(out / "receipt.json", result)
print(json.dumps({**{key: result[key] for key in ("requests", "same_page", "peak_queue", "journal_allocations")},
                  "up_to_4":sum(buckets[:5]),"up_to_8":sum(buckets[:9]),"up_to_16":sum(buckets[:17]),
                  "up_to_32":sum(buckets[:33])}, indent=2))
