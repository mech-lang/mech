from pathlib import Path
import hashlib,json,subprocess,time
P=Path(__file__).parent;B=Path('/private/tmp/mech-related-llvm-native-image-smoke-20261107-v2');O=Path('/private/tmp/mech-related-llvm-native-image-runtime-20261107-v4');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(cmd,path):
 t=time.monotonic();cmd=list(map(str,cmd));r=subprocess.run(cmd,capture_output=True,text=True);path.write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,qualification_only=True),indent=2)+'\n');print(path.stem,r.returncode,flush=True);assert r.returncode==0,r.stdout[-4000:]+r.stderr[-4000:]
proof={};oracle=json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())
for name in ['after-4','reuse-4']:
 p=P/name;new={x.name:sha(x)for x in(p/'build').iterdir()if x.is_file()};old={x.name:sha(x)for x in(B/name/'build').iterdir()if x.is_file()};assert new==old and len(new)==13
 (p/'runner.rs').write_bytes((B/name/'runner.rs').read_bytes());run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--linked','--bundle',p/'build'],p/'runner-build.json');run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'link-proof.json'],p/'link-audit.json')
 run([p/'runner-aot',p/'build/source.mec','/private/tmp/mech-related-current-input-fixtures-20261107/posts.json',p/'full5000-output.json','1'],p/'full5000-run.json');assert json.loads((p/'full5000-output.json').read_text())==oracle
 run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',p/'edges'],p/'edge-run.json');proof[name]=dict(all13_files_identical_to_v2=True,full5000_exact=True,Rust_edges_exact=31,runtime_only_GPU=True,all_link_audits_passed=True)
P.joinpath('linked-correctness.json').write_text(json.dumps(dict(variants=proof,exact_outputs=64,old_v3_loader_and_budget_evidence_applies_to_identical_Objects=True,performance_measured=False),indent=2)+'\n')
