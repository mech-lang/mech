"""Stage the pinned grammar and diagnostic-only source hooks without compiling."""
from hashlib import sha1, sha256
import json
from pathlib import Path
import shutil
import subprocess

study = Path(__file__).resolve().parent
root = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP")
qualification = root / ".qualification"
checkout = qualification / "native-review-pr/checkout"
pin = "dae46c686703560230150a93f36c44c4978ffdc7"
syntax = Path("src/syntax/src")
document = Path("document/parser/canonical/document")
base = Path("/private/tmp/mech-dense-index.5noolrp2")
base_source = base / "candidate/source"
control = base / "candidate/parser"
receipt = json.loads((base / "build.json").read_text())
source_rel = Path("benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec")
source = checkout / source_rel
source_dir = study / "source"


def git(*args):
    return subprocess.check_output(["git", "-C", root, *map(str, args)])


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def verify():
    records = git("ls-tree", "-r", "-z", pin, syntax).split(b"\0")
    blobs = {}
    for record in filter(None, records):
        parts, raw_path = record.split(b"\t")
        _, obj = parts.split()[1:]
        path = Path(raw_path.decode()).relative_to(syntax)
        data = (base_source / path).read_bytes()
        actual = sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
        assert actual == obj.decode(), path
        blobs[str(path)] = actual
    assert len(blobs) == 164
    assert set(blobs) == {str(path.relative_to(base_source)) for path in base_source.rglob("*") if path.is_file()}
    assert source.read_bytes() == git("show", f"{pin}:{source_rel}")
    assert digest(source) == receipt["exact_tree_and_work"]["source_sha256"]
    assert digest(control) == receipt["probe_sha256"]
    allocator = Path("src/compiler_allocator.rs")
    assert git("show", f"{pin}:{allocator}") == (checkout / allocator).read_bytes()
    return blobs


def replace(text, original, replacement):
    assert text.count(original) == 1, original
    return text.replace(original, replacement)


def patch_continuation(text):
    text = replace(text, "    pub peak_frames: usize,\n}", """    pub peak_frames: usize,
    best_choice_probe: super::best_choice_probe::State,
}""")
    text = replace(text, "            peak_frames: 1,\n", """            peak_frames: 1,
            best_choice_probe: super::best_choice_probe::State::default(),
""")
    text = replace(text, """    ) -> Progress {
        while !self.frames.is_empty()""", """    ) -> Progress {
        self.best_choice_probe.resume();
        let progress = self.advance_frames(parser, input_final, allowance);
        self.best_choice_probe.suspend();
        progress
    }

    fn advance_frames(
        &mut self,
        parser: &mut Parser<'_>,
        input_final: bool,
        allowance: &mut u64,
    ) -> Progress {
        while !self.frames.is_empty()""")
    completed = """self.best_choice_probe.finish_ready(
                self.frames.len(), parser, self.state, self.steps + self.child_work, input_final,
                self.result,
            );"""
    text = replace(text, "            let final_input = input_final || !parser.state.cursor_frontier;\n",
                   f"            {completed}\n            let final_input = input_final || !parser.state.cursor_frontier;\n")
    text = replace(text, "        Progress::Complete(self.result)\n    }\n", f"        {completed}\n        Progress::Complete(self.result)\n    }}\n")
    text = replace(text, "                Frame::BestChoice(mut frame, mut selected) => {\n", """                Frame::BestChoice(mut frame, mut selected) => {
                    let work = self.steps + self.child_work;
                    let mut wins = false;
""")
    text = replace(text, """                    if parser.is_halted() {
                        self.result = Attempt::Committed;
                        continue;
                    }
                    if self.result == Attempt::Committed && selected.is_none() {
                        continue;
                    }
                    if self.result == Attempt::Matched {
""", """                    if parser.is_halted() {
                        self.best_choice_probe.trial(parser, self.state, work - 1, input_final, self.result, false);
                        self.best_choice_probe.stop_choice();
                        self.result = Attempt::Committed;
                        continue;
                    }
                    if self.result == Attempt::Committed && selected.is_none() {
                        self.best_choice_probe.trial(parser, self.state, work - 1, input_final, self.result, false);
                        self.best_choice_probe.stop_choice();
                        continue;
                    }
                    if self.result == Attempt::Matched {
""")
    text = replace(text, """                            selected = Some((frame.index, consumed));
                        }
                    }
                    frame.index += 1;""", """                            selected = Some((frame.index, consumed));
                            wins = true;
                        }
                    }
                    self.best_choice_probe.trial(parser, self.state, work - 1, input_final, self.result, wins);
                    frame.index += 1;""")
    text = replace(text, """                        self.push(Frame::BestChoice(frame, selected));
                        self.push(Frame::Expression(item));
                    } else if let Some((index, _)) = selected {
                        self.push(Frame::Expression(&frame.items[index]));
                    } else {
                        self.result = Attempt::NoMatch;
""", """                        self.best_choice_probe.begin_trial(parser, self.state, work, input_final);
                        self.push(Frame::BestChoice(frame, selected));
                        self.push(Frame::Expression(item));
                    } else if let Some((index, _)) = selected {
                        let expression = &frame.items[index];
                        self.best_choice_probe.replay(self.frames.len(), expression, parser, self.state, work, input_final);
                        self.push(Frame::Expression(expression));
                    } else {
                        self.best_choice_probe.stop_choice();
                        self.result = Attempt::NoMatch;
""")
    text = replace(text, "                Frame::Expression(expression) => self.expression(parser, expression),",
                   "                Frame::Expression(expression) => self.expression(parser, expression, input_final),")
    text = replace(text, """    fn expression(&mut self, parser: &mut Parser<'_>, expression: &'g GrammarExpression) {""",
                   """    fn expression(&mut self, parser: &mut Parser<'_>, expression: &'g GrammarExpression, input_final: bool) {""")
    return replace(text, """                    parser.rewind(frame.checkpoint);
                    self.push(Frame::BestChoice(frame, None));""", """                    parser.rewind(frame.checkpoint);
                    self.best_choice_probe.begin_choice(parser, self.state, self.steps + self.child_work, input_final);
                    self.push(Frame::BestChoice(frame, None));""")


def patch_document(text):
    text = replace(text, "pub(crate) mod continuation;\n", "mod best_choice_probe;\npub(crate) mod continuation;\n")
    text = replace(text, "    let mut continuation = continuation::Continuation::document_root();\n", """    let mut continuation = continuation::Continuation::document_root();
    best_choice_probe::begin();
""")
    return replace(text, "            continuation::Progress::Complete(_) => return,\n", """            continuation::Progress::Complete(_) => {
                best_choice_probe::report(parser, continuation.steps, continuation.child_work, continuation.peak_frames);
                return;
            }
""")


def prepare():
    blobs = verify()
    source_dir.mkdir(exist_ok=False)
    shutil.copytree(base_source, source_dir, dirs_exist_ok=True)
    cont = source_dir / document / "continuation.rs"
    mod = source_dir / document / "mod.rs"
    cont.write_text(patch_continuation(cont.read_text()))
    mod.write_text(patch_document(mod.read_text()))
    probe = source_dir / document / "best_choice_probe.rs"
    shutil.copyfile(study / "best_choice_probe.rs", probe)
    subprocess.run([
        "rustfmt", "+nightly-2026-03-03", "--edition", "2024",
        "--config", "skip_children=true", cont, mod, probe,
    ], check=True)
    print(f"Pinned {len(blobs)} syntax blobs and the exact source and control at {pin}.")
    (study / "source-bindings.json").write_text(json.dumps({
        "pin": pin,
        "pinned_inputs": receipt["exact_tree_and_work"],
        "base_probe_sha256": digest(control),
        "base_library_sha256": receipt["library_sha256"],
        "source_blobs": blobs,
    }, indent=2) + "\n")


if __name__ == "__main__":
    prepare()
