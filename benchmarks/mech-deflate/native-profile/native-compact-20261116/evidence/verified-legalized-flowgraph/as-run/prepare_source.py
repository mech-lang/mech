"""Prepare an independent flowgraph storage reuse experiment."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import shutil
import subprocess

study = Path(__file__).resolve().parent
q = study.parent
root = q.parent
checkout = q / "native-review-pr/checkout"
parent = "197ad8ff1272a9f411bd56f14f08efabba2553cd"
assert subprocess.check_output(["git", "rev-parse", "refs/heads/codex/native-compact-source-handoff"],
                               cwd=root, text=True).strip() == parent


def tracked(relative):
    return subprocess.check_output(["git", "show", f"{parent}:{relative}"], cwd=root)


def digest(file):
    with file.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def replace(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new, 1)


prefix = Path("vendor/cranelift-codegen/src")
context = prefix / "context.rs"
verifier = prefix / "verifier/mod.rs"
ctx = tracked(context).decode()
assert (checkout / context).read_text() == ctx
ctx = replace(ctx, "use crate::verifier::{VerifierErrors, VerifierResult, verify_context};",
              "use crate::verifier::{VerifierErrors, VerifierResult, verify_context, verify_function_and_flowgraph};")
ctx = replace(ctx, """        self.legalize(isa)?;

        self.compute_cfg();
        self.compute_domtree();
""", """        self.legalize_ir(isa);
        if isa.flags().enable_verifier() {
            // The verifier builds both graphs from the legalized function.
            // Keep those graphs for the next pass instead of rebuilding them.
            (self.cfg, self.domtree) = verify_function_and_flowgraph(&self.func, isa)?;
        } else {
            self.flowgraph();
        }
""")
ctx = replace(ctx, """    pub fn legalize(&mut self, isa: &dyn TargetIsa) -> CodegenResult<()> {
        // Legalization invalidates the domtree and loop_analysis by mutating the CFG.
""", """    pub fn legalize(&mut self, isa: &dyn TargetIsa) -> CodegenResult<()> {
        self.legalize_ir(isa);
        self.verify_if(isa)
    }

    fn legalize_ir(&mut self, isa: &dyn TargetIsa) {
        // Legalization invalidates the domtree and loop_analysis by mutating the CFG.
""")
ctx = replace(ctx, """        simple_legalize(&mut self.func, isa);
        self.verify_if(isa)
""", """        simple_legalize(&mut self.func, isa);
""")

verify = tracked(verifier).decode()
assert (checkout / verifier).read_text() == verify
verify = replace(verify, """    let _tt = timing::verifier();
    let mut errors = VerifierErrors::default();
    let verifier = Verifier::new(func, fisa.into());
    let result = verifier.run(&mut errors);
    if errors.is_empty() {
        result.unwrap();
        Ok(())
    } else {
        Err(errors)
    }
}
""", """    verify_function_and_flowgraph(func, fisa).map(|_| ())
}

/// Verify the function and return the graphs built directly from its IR.
pub(crate) fn verify_function_and_flowgraph<'a, FOI: Into<FlagsOrIsa<'a>>>(
    func: &Function,
    fisa: FOI,
) -> Result<(ControlFlowGraph, DominatorTree), VerifierErrors> {
    let _tt = timing::verifier();
    let mut errors = VerifierErrors::default();
    let verifier = Verifier::new(func, fisa.into());
    let result = verifier.run(&mut errors);
    if errors.is_empty() {
        result.unwrap();
        Ok((verifier.expected_cfg, verifier.expected_domtree))
    } else {
        Err(errors)
    }
}
""")
for kind, files in (("before", [(context, tracked(context).decode()), (verifier, tracked(verifier).decode())]),
                    ("candidate", [(context, ctx), (verifier, verify)])):
    for relative, text in files:
        target = study / kind / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

route = Path("hosts/gpu/src/native_artifact/program/codegen/cfg_cleanup/identity_routes/route.rs")
dominance = q / "native-dominance-search"
compiler = q / "host-opt-levels/opt3-cargo/release/compile_profile"
timing = json.loads((dominance / "matched/timing.json").read_text())
assert digest(compiler) == timing["compiler_sha256"]["mech-experiment"]
shutil.copy2(compiler, dominance / "candidate-compile_profile")
current_route, original_route = ((dominance / f"route-{kind}.rs").read_bytes() for kind in ("after", "before"))
assert (checkout / route).read_bytes() == current_route and tracked(route) == original_route
tmp_route = (checkout / route).with_suffix(".restore")
tmp_route.write_bytes(original_route)
os.replace(tmp_route, checkout / route)
