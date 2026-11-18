"""Count cache stack growth on separate instrumented parser executables."""

import json
import shutil

from study_tools import build, link, probe, replace, study, write

census = study / "frames"
census.mkdir()
counts = {}
for role in ("before", "scratch"):
    output = census / role
    src = output / "source"
    shutil.copytree(study / role / "source", src)
    cache = src / "document/parser/tree_cache.rs"
    text = cache.read_text()
    text = replace(text, "                        self.frames.push(Frame {", """                        STACKS.with(|counts| {
                            let (pushes, growths, max_depth) = counts.get();
                            let depth = self.frames.len();
                            counts.set((pushes + 1, growths + u64::from(depth == self.frames.capacity()),
                                        max_depth.max(depth + 1)));
                        });
                        self.frames.push(Frame {""")
    text += """
std::thread_local! {
    static STACKS: core::cell::Cell<(u64, u64, usize)> =
        const { core::cell::Cell::new((0, 0, 0)) };
}

pub(super) fn print_stack_counts() {
    STACKS.with(|counts| {
        let (pushes, growths, max_depth) = counts.get();
        eprintln!("cache-stack-counts|{pushes}|{growths}|{max_depth}");
    });
}
"""
    cache.write_text(text)
    parser = cache.with_name("mod.rs")
    text = replace(parser.read_text(), "        let state = self.suspend();\n        ParserOutput {",
                   "        tree_cache::print_stack_counts();\n        let state = self.suspend();\n        ParserOutput {")
    parser.write_text(text)
    lib, compile = build(output, src, f"parser-frame-census-{role}")
    executable, linker = link(output, lib)
    result = probe(executable, output / "parse.json")
    log = (output / "parse.log").read_text()
    entry, = [line for line in log.splitlines() if line.startswith("cache-stack-counts|")]
    _, pushes, growths, depth = entry.split("|")
    counts[role] = {
        "frame_pushes": int(pushes), "frame_capacity_growths": int(growths), "maximum_frame_depth": int(depth),
        "compiler": compile, "link": linker, "probe_tree_sha256": result["tree_sha256"],
    }
assert counts["before"]["frame_pushes"] == counts["scratch"]["frame_pushes"]
assert counts["before"]["maximum_frame_depth"] == counts["scratch"]["maximum_frame_depth"]
assert counts["scratch"]["frame_capacity_growths"] < counts["before"]["frame_capacity_growths"]
write(census / "receipt.json", counts)
print({role: {key: n for key, n in row.items() if isinstance(n, int)} for role, row in counts.items()})
