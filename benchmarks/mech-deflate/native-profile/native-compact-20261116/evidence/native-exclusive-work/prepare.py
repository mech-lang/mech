"""Prepare a private diagnostic patch and a one-change facts-owner ablation."""

from difflib import unified_diff
from hashlib import sha256
import json
from pathlib import Path
import subprocess


HERE = Path(__file__).resolve().parent
REPO = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP")
BASE = "dae46c686703560230150a93f36c44c4978ffdc7"
ENGINE = "src/engine/src/native_artifact/"
GPU = "hosts/gpu/src/native_artifact/program/"
CRANELIFT = "vendor/cranelift-codegen/src/"
DRIVER = "benchmarks/mech-deflate/native-profile/compile_profile.rs"
USE = "use mech_engine::native_artifact::compiler_trace as trace;\n"
sources = {}
originals = {}


def read(path):
    if path not in originals:
        originals[path] = subprocess.check_output(["git", "show", f"{BASE}:{path}"], cwd=REPO).decode()
    return sources.get(path, originals[path])


def replace(path, before, after):
    code = read(path)
    assert code.count(before) == 1, (path, before)
    sources[path] = code.replace(before, after)


def before(path, old, insertion):
    replace(path, old, insertion + old)


def after(path, old, insertion):
    replace(path, old, old + insertion)


def span(variable, category, detail, indent=4):
    return " " * indent + f'let {variable} = trace::span("{category}", "{detail}");\n'


def close(variable):
    return f"    drop({variable});\n"


def add_trace_import(path):
    code = read(path)
    index = code.index("\nuse ") + 1
    sources[path] = code[:index] + USE + code[index:]


def fact_probes():
    mod_path = ENGINE + "mod.rs"
    after(mod_path, "mod control;\n", "#[doc(hidden)]\npub mod compiler_trace;\n")
    new_path = ENGINE + "compiler_trace.rs"
    originals[new_path] = ""
    sources[new_path] = (HERE / "compiler_trace.rs").read_text()
    facts = ENGINE + "program_facts.rs"
    before(facts, "use super::{BAD", "use crate::native_artifact::compiler_trace as trace;\n")
    after(facts, "        self.direct_outputs.get_or_init(|| {\n",
          span("_time", "immutable_facts", "direct action outputs", 12))
    before(facts, "                let mut reads = vec![Vec::new(); self.plan().values().len()];\n",
           span("_time", "immutable_facts", "read roles", 16))
    after(facts, "        self.graph\n            .get_or_init(|| {\n",
          span("_time", "immutable_facts", "graph", 16))
    for value, detail in (("definitions(self.plan())", "definitions"),
                          ("super::reads(self.plan(), self.actions()?)", "fusion reads"),
                          ("crate::native_artifact::scalar_aggregates::derive(self.plan(), self)", "scalar aggregates")):
        replace(facts, f"            .get_or_init(|| {value})",
                "            .get_or_init(|| {\n" + span("_time", "immutable_facts", detail, 16)
                + f"                {value}\n            }})")
    sem = ENGINE + "fusion_semantics.rs"
    for fn, detail in (("semantic_use_counts", "use counts"), ("semantic_actions", "semantic actions")):
        replace(sem, f"            .get_or_init(|| {fn}(self.plan))",
                "            .get_or_init(|| {\n"
                + f'                let _time = crate::native_artifact::compiler_trace::span("immutable_facts", "{detail}");\n'
                + f"                {fn}(self.plan)\n            }})")


def cranelift_hooks():
    replace("hosts/gpu/Cargo.toml", 'features = ["experimental-cmp-reuse"]',
            'features = ["experimental-cmp-reuse", "timing"]')
    after(CRANELIFT + "timing.rs", '    compile: "Compilation passes",\n',
          '    context_optimize: "Optimize IR in Context",\n'
          '    target_compile: "Compile for the target ISA",\n')
    path = CRANELIFT + "context.rs"
    replace(path, "            result = isa.compile_function(&self.func, &self.domtree, self.want_disasm, ctrl_plane);\n",
            "            let _target = timing::target_compile();\n"
            "            result = isa.compile_function(&self.func, &self.domtree, self.want_disasm, ctrl_plane);\n")
    before(path, '        log::debug!(\n            "Number of CLIF instructions to optimize: {}",',
           "        let _optimize = timing::context_optimize();\n")


def compiler_entry():
    p = GPU + "compiler.rs"
    add_trace_import(p)
    before(p, "fn emit_compact_closed_object(\n", '''struct CraneliftTiming;

impl cranelift_codegen::timing::Profiler for CraneliftTiming {
    fn start_pass(&self, pass: cranelift_codegen::timing::Pass) -> Box<dyn std::any::Any> {
        use cranelift_codegen::timing::Pass;
        let category = match pass {
            Pass::context_optimize => "cranelift_optimize",
            Pass::target_compile => "cranelift_lower_codegen",
            _ => "cranelift_pass",
        };
        Box::new(trace::span(category, pass.description()))
    }
}

pub(super) struct InstalledTiming(Box<dyn cranelift_codegen::timing::Profiler>);

impl Drop for InstalledTiming {
    fn drop(&mut self) {
        let prior = std::mem::replace(
            &mut self.0,
            Box::new(cranelift_codegen::timing::DefaultProfiler),
        );
        drop(cranelift_codegen::timing::set_thread_profiler(prior));
    }
}

pub(super) fn install_compiler_timing() -> Option<InstalledTiming> {
    trace::session().map(|_| {
        InstalledTiming(cranelift_codegen::timing::set_thread_profiler(Box::new(CraneliftTiming)))
    })
}

''')
    before(p, '    eprintln!("compact-stage: canonical lowering");\n',
           '    let _cranelift_timing = install_compiler_timing();\n'
           + span("plan_work", "native_plan", "lower artifact"))
    before(p, '    eprintln!(\n        "compact-stage: preparation blocks={} values={}",\n',
           close("plan_work") + span("semantic_work", "semantic_preparation", "prepare native program"))
    before(p, '    eprintln!("compact-stage: native module");\n',
           close("semantic_work") + span("module_setup", "phase_selection", "module setup"))
    before(p, "    let metadata = if let Some(toolchain) = llvm_root {\n", close("module_setup"))
    before(p, '    if !observe && std::env::var_os("MECH_COMPACT_EXPORTS_FIRST").is_some() {\n',
           span("_publish", "object_publication", "finish object"))


def timed_pool():
    p = GPU + "compact/compile_jobs.rs"
    add_trace_import(p)
    replace(p, "pub(super) struct Ticket<T = Context>(mpsc::Receiver<Result<T, NativeCompileError>>);",
            "pub(super) struct Ticket<T = Context>(mpsc::Receiver<Result<T, NativeCompileError>>, usize);")
    before(p, '            let worker = std::thread::Builder::new()\n',
           '            let session = trace::session();\n')
    replace(p, '''                .spawn(move || {
                    loop {
                        let next = { jobs.lock().unwrap().recv() };
''', '''                .spawn(move || {
                    let _thread = session.map(|session| trace::attach(session, format!("worker-{index}")));
                    let _backend = crate::native_artifact::program::compiler::install_compiler_timing();
                    let _lifetime = trace::span("worker", "worker lifetime");
                    loop {
                        let next = {
                            let _wait = trace::span("worker_wait", "queue receive and lock");
                            jobs.lock().unwrap().recv()
                        };
''')
    replace(p, "        self.enqueue(move |isa| compile(context, isa))", '        self.enqueue("compile", move |isa| compile(context, isa))')
    replace(p, "        let ticket = self.enqueue(work)?;", '        let ticket = self.enqueue("cleanup", work)?;')
    after(p, "    fn enqueue<T: Send + 'static>(\n        &self,\n", "        kind: &'static str,\n")
    before(p, "        self.queue\n            .as_ref()\n            .unwrap()\n            .send(Box::new(move |isa| {\n",
           '        let job = self.submitted.get();\n'
           '        let enqueue = trace::job_span("coordinator_wait", "enqueue", job);\n')
    replace(p, '''                let _ = reply.send(work(isa));
            }))
            .map_err(backend)?;
        self.submitted.set(self.submitted.get() + 1);
        Ok(Ticket(result))''', '''                let _job = trace::job_span("worker_job", kind, job);
                let compute = trace::span("worker_compute", kind);
                let output = work(isa);
                drop(compute);
                let _reply = trace::span("coordination", "worker reply");
                let _ = reply.send(output);
            }))
            .map_err(backend)?;
        drop(enqueue);
        self.submitted.set(self.submitted.get() + 1);
        Ok(Ticket(result, job))''')
    before(p, "        self.queue.take();\n", span("_wait", "coordinator_wait", "close queue and join workers", 8))
    before(p, "        self.0.recv().map_err(backend)?\n",
           '        let _wait = trace::job_span("coordinator_wait", "receive completion", self.1);\n')
    before(p, "        let mut declarations = ModuleDeclarations::default();\n",
           span("_snapshot", "coordination", "snapshot recorded module declarations", 8))
    before(p, "        for event in self.events {\n", span("_replay", "coordination", "replay recorded module", 8))
    before(p, "    let code = context\n        .compiled_code()\n", span("_publish", "object_publication", "publish compiled function"))


def codegen_spans():
    p = GPU + "compact/emit.rs"
    add_trace_import(p)
    before(p, "    let layout = Layout::derive(plan)?;\n",
           span("shared_prep", "semantic_preparation", "typed layout and operation recipes"))
    before(p, "    let phases = phases::define(\n", close("shared_prep"))
    before(p, '    eprintln!("compact-stage: typed layout");\n', span("root_prep", "semantic_preparation", "checked root flow and recipes"))
    before(p, "        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);\n",
           "        drop(root_prep);\n" + span("_ssa", "ssa_construction", "checked root", 8))
    before(p, "    if let Some(growth) = capacity_growth {\n", span("cleanup_time", "cfg_cleanup", "checked root"))
    before(p, '    eprintln!("compact-stage: machine code");\n', close("cleanup_time"))
    before(p, '    eprintln!("compact-stage: machine code");\n', span("backend_time", "codegen_context", "checked root"))
    after(p, '    eprintln!("compact-stage: machine code complete");\n', close("backend_time"))
    p = GPU + "compact/phase_function.rs"
    add_trace_import(p)
    after(p, ") -> Result<(Context, numeric_compact::Report), NativeCompileError> {\n",
          span("_ssa", "ssa_construction", "numeric phase"))
    after(p, ") -> Result<PhaseCleanup, NativeCompileError> {\n", span("_time", "cfg_cleanup", "numeric phase"))
    replace(p, '''            && receiver
                .recv()
                .is_ok_and(|general| general == vector.context.func)
''', '''            && {
                let wait = trace::span("coordinator_wait", "receive prepared IR");
                let general = receiver.recv();
                drop(wait);
                let _compare = trace::span("coordination", "compare prepared IR");
                general.is_ok_and(|general| general == vector.context.func)
            }
''')
    after(p, "                    if let Some(sender) = sender {\n",
          span("_transfer", "coordination", "publish prepared IR", 24))
    p = GPU + "compact/phases.rs"
    add_trace_import(p)
    before(p, "    let mut candidates = Vec::new();\n", span("discovery", "phase_selection", "candidate discovery and sort"))
    before(p, "    let mut covered = BTreeSet::new();\n", close("discovery"))
    after(p, "    for (id, private) in candidates {\n", span("selection", "phase_selection", "phase packets and guarded entries", 8))
    before(p, "        let (ready, general_declaration, entry_bounds, vector_contract) =\n", "        drop(selection);\n")
    p = GPU + "compact/object.rs"
    add_trace_import(p)
    before(p, "        let preselected = checked_root\n", span("compile_body", "codegen_context", "assemble body", 8))
    before(p, "        let buffer = &context.compiled_code().unwrap().buffer;\n        let mut bytes = buffer.data().into();\n",
           "        drop(compile_body);\n" + span("_publish", "object_publication", "machine body and relocations", 8))


def driver():
    add_trace_import(DRIVER)
    after(DRIVER, '    eprintln!("compile-profile: phase=native mode={}", args[3]);\n', '''    let profile = std::env::var_os("MECH_NATIVE_EXCLUSIVE_TRACE").map(|filename| {
        assert_eq!(args[3], "compact");
        let session = trace::Session::new();
        let coordinator = trace::attach(session.clone(), "coordinator".into());
        (session, coordinator, filename)
    });
''')
    before(DRIVER, '    let (object, metadata) = match args[3].as_str() {\n',
           span("native_span", "native", "artifact through object bytes"))
    before(DRIVER, '    stamp("native", start);\n', close("native_span"))
    after(DRIVER, '    let total_seconds = total.elapsed().as_secs_f64();\n', '''    if let Some((session, coordinator, filename)) = profile {
        drop(coordinator);
        let file = std::fs::OpenOptions::new().write(true).create_new(true).open(filename).unwrap();
        serde_json::to_writer(file, &session.take_reports()).unwrap();
    }
''')


def patch(output, old, new):
    chunks = []
    for path, body in sorted(new.items()):
        prior = old[path]
        chunk = unified_diff(prior.splitlines(keepends=True), body.splitlines(keepends=True),
                             fromfile=f"a/{path}" if prior else "/dev/null", tofile=f"b/{path}")
        chunks.append("".join(chunk))
    (HERE / output).write_text("".join(chunks))


def run():
    fact_probes()
    cranelift_hooks()
    compiler_entry()
    timed_pool()
    codegen_spans()
    driver()
    patch("trace.patch", originals, sources)
    owner = GPU + "codegen/numeric_compact.rs"
    base = read(owner)
    start = "    let read_packs = super::numeric_read_packs::derive(&program.metadata().plan.facts(), proof);\n"
    late = "    let boolean_successors =\n        super::numeric_branches::derive(&program.metadata().plan.facts(), proof);\n"
    combined = '''    let (read_packs, boolean_successors) = {
        let facts = program.metadata().plan.facts();
        (
            super::numeric_read_packs::derive(&facts, proof),
            super::numeric_branches::derive(&facts, proof),
        )
    };
'''
    assert base.count(start) == base.count(late) == 1
    shared = base.replace(start, combined).replace(late, "")
    patch("shared-facts-only.patch", {owner: base}, {owner: shared})
    print(f"{len(sources)} trace-only files and one facts-owner file at {BASE}")
    receipt = {"base": BASE, "inputs": {path: sha256(body.encode()).hexdigest() for path, body in sorted(originals.items())},
               "patches": {name: sha256((HERE / name).read_bytes()).hexdigest()
                           for name in ("trace.patch", "shared-facts-only.patch")}}
    (HERE / "plan-inputs.json").write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    run()
