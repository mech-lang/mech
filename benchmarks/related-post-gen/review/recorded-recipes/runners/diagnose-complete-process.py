from pathlib import Path
import hashlib,json,statistics,subprocess,time
U=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
P=Path('/private/tmp/mech-related-native-fixed-frame-arenas-complete-phase-diagnostic-20261106-v1');P.mkdir(exist_ok=True)
config=json.loads((U/'comparison-config.json').read_text());sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
oracle=json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())
names=['mech-parent-4','mech-after-4','mech-u8-ids-u32-8'];rows={r['name']:r for r in config['implementations']if r['name']in names}
extra='''
static PHASE_ORIGIN: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
static PHASE_MARKS: [std::sync::atomic::AtomicU64; 10] = [const { std::sync::atomic::AtomicU64::new(0) }; 10];
fn phase_mark(slot: usize) { PHASE_MARKS[slot].store(PHASE_ORIGIN.get().unwrap().elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed); }
struct PhaseDrop(usize);
impl Drop for PhaseDrop { fn drop(&mut self) { phase_mark(self.0); } }
'''
def run(cmd,path):
 start=time.monotonic();r=subprocess.run(list(map(str,cmd)),capture_output=True,text=True);x=dict(command=list(map(str,cmd)),exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr);path.write_text(json.dumps(x,indent=2)+'\n');assert r.returncode==0,r.stderr;return x
for name in names:
 row=rows[name];receipt=json.loads(Path(row['receipt']).read_text());b=Path(row['executable']).parent;p=P/name;p.mkdir(exist_ok=True);(p/'build').symlink_to(b/'build',target_is_directory=True)
 s=(b/'runner.rs').read_text();s=s.replace('fn run() {',extra+'\nfn run() {\n    phase_mark(1);\n    let _after_all_local_drops = PhaseDrop(7);',1)
 s=s.replace('    let mode = NativeProgramLimitMode::NoLimits;','    phase_mark(9);\n    let mode = NativeProgramLimitMode::NoLimits;',1)
 line='            serde_json::to_writer(BufWriter::new(File::create(&args[2]).unwrap()), &related).unwrap();';assert s.count(line)==1
 s=s.replace(line,'            phase_mark(2);\n'+line+'\n            phase_mark(3);')
 s=s.replace('    drop(posts_view);','    phase_mark(4);\n    drop(posts_view);',1).replace('    drop(input);','    drop(input);\n    phase_mark(5);',1)
 s=s.replace('    assert_eq!(memory.retained_snapshot_nodes(), 0);','    assert_eq!(memory.retained_snapshot_nodes(), 0);\n    phase_mark(6);',1)
 s=s.replace('fn main() {','fn main() {\n    PHASE_ORIGIN.set(Instant::now()).unwrap();\n    phase_mark(0);',1)
 line='    std::thread::Builder::new().stack_size(32*1024*1024).spawn(run).unwrap().join().unwrap();';assert s.count(line)==1
 s=s.replace(line,line+'\n    phase_mark(8);\n    let marks: Vec<u64> = PHASE_MARKS.iter().map(|v|v.load(std::sync::atomic::Ordering::Relaxed)).collect();\n    eprintln!("{}", serde_json::json!({"diagnostic_phase_marks_ns":marks}));')
 (p/'runner.rs').write_text(s);run(['python3',R/'build-host.py','--compiler-build',receipt['compiler_build'],'--source',p/'runner.rs','--out',p/'runner-phases','--bundle',p/'build','--linked'],p/'build-run.json');run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-phases','--output',p/'native-link-audit.json'],p/'audit-run.json')
samples=[]
for i in range(9):
 for name in names if i%2==0 else names[::-1]:
  p=P/name;output=p/'output.json';x=run([p/'runner-phases',p/'build/source.mec','/private/tmp/mech-related-post-rust-local-20261102/posts.json',output,'1'],p/f'run-{i}.json');assert json.loads(output.read_text())==oracle
  timing=json.loads(x['stdout']);m=json.loads(x['stderr'])['diagnostic_phase_marks_ns'];ns=1e9
  phases=dict(thread_start=(m[1]-m[0])/ns,catalog_and_source=(m[9]-m[1])/ns,restore=timing['compile_or_restore_seconds'],json=timing['json_read_and_parse_seconds'],post_json=timing['post_json_seconds'],json_output=(m[3]-m[2])/ns,output_retirement=(m[4]-m[3])/ns,input_retirement=(m[5]-m[4])/ns,remaining_local_drops=(m[7]-m[6])/ns,thread_finish_join=(m[8]-m[7])/ns,main_to_join=m[8]/ns,outside_main_and_observer=x['seconds']-m[8]/ns)
  samples.append(dict(variant=name,sample=i,wall_seconds=x['seconds'],output_sha256=sha(output),exact_standard_rust_output=True,phase_seconds=phases))
summary={n:{k:statistics.median(r['phase_seconds'][k]for r in samples if r['variant']==n)for k in samples[0]['phase_seconds']}for n in names}
result=dict(schema='mech.related-post-gen.complete-process-phase-diagnostic.v1',diagnostic_timings_excluded=True,limitations=['Instrumented host executables are diagnostic, not the latency comparison binaries.','Outside-main includes launch/exit, process observation, pipe collection, and stderr diagnostic serialization; it is not a Mech CPU-time attribution.','Automatic cleanup markers use only atomic stores; ordinary object destruction order is preserved.','Separate medians are not additive.'],all27_outputs_exact=True,samples=samples,summary=summary)
(P/'summary.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(summary),flush=True)
