from pathlib import Path
import hashlib,json,os,random,re,statistics,subprocess,time
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');I=Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json');oracle=json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())
snapshot=subprocess.check_output(['ps','-axo','pid,ppid,etime,pcpu,command'],text=True);busy=[]
for line in snapshot.splitlines()[1:]:
 parts=line.split(None,4)
 if len(parts)!=5 or int(parts[0])in[os.getpid(),os.getppid()]:continue
 if re.search(r'(?:^|/)(?:rustc|cargo|clang|clang\+\+|llc|opt|sample|gzip|xz|zstd|gpu-tests|one-kib|scalar|runner-(?:aot|metrics|pc|trace|phases))(?:\s|$)',parts[4]):busy.append(dict(pid=int(parts[0]),program=parts[4].split()[0]))
assert not busy,busy
(P/'quiet-start.json').write_text(json.dumps(dict(quiet=True,process_snapshot_sha256=hashlib.sha256(snapshot.encode()).hexdigest(),rounds=9,seed=99027301,diagnostic_only=True),indent=2)+'\n')
rows=[];rng=random.Random(99027301)
for round in range(9):
 names=['mech-ir242-reuse-4','mech-ir257-reuse-4'];rng.shuffle(names)
 for name in names:
  p=P/name;out=p/('output-'+str(round)+'.json');cmd=[str(p/'runner-trace'),str(p/'build/source.mec'),str(I),str(out),'1'];t=time.perf_counter();r=subprocess.run(cmd,capture_output=True,text=True);wall=time.perf_counter()-t;assert r.returncode==0,r.stderr;assert json.loads(out.read_text())==oracle;trace=json.loads(next(x.removeprefix('LIFECYCLE_TRACE ')for x in r.stdout.splitlines()if x.startswith('LIFECYCLE_TRACE ')));row=dict(round=round,variant=name,command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr,wall_seconds=wall,trace=trace,launch_trace_output_and_exit_residue_seconds=wall-trace['main_to_join_seconds'],exact_Rust_output=True,diagnostic_only=True);rows.append(row);(p/('run-'+str(round)+'.json')).write_text(json.dumps(row,indent=2)+'\n');out.unlink()
 (P/'rows.json').write_text(json.dumps(rows,indent=2)+'\n');print('lifecycle round',round+1,'exact',flush=True)
summary={}
for name in ['mech-ir242-reuse-4','mech-ir257-reuse-4']:
 a=[x for x in rows if x['variant']==name];phases={key:statistics.median(dict(x['trace']['phases'])[key]for x in a)for key,_ in a[0]['trace']['phases']};summary[name]=dict(phases_median_seconds=phases,wall_median_seconds=statistics.median(x['wall_seconds']for x in a),thread_start_median_seconds=statistics.median(x['trace']['host_thread_start_seconds']for x in a),thread_join_finalize_median_seconds=statistics.median(x['trace']['host_thread_join_finalize_seconds']for x in a),launch_trace_output_and_exit_residue_median_seconds=statistics.median(x['launch_trace_output_and_exit_residue_seconds']for x in a))
paired=[]
for round in range(9):
 a=next(x for x in rows if x['round']==round and x['variant']=='mech-ir242-reuse-4');b=next(x for x in rows if x['round']==round and x['variant']=='mech-ir257-reuse-4');paired.append(dict(round=round,wall_delta=b['wall_seconds']-a['wall_seconds'],phases={k:dict(b['trace']['phases'])[k]-v for k,v in a['trace']['phases']},thread_start=b['trace']['host_thread_start_seconds']-a['trace']['host_thread_start_seconds'],thread_join_finalize=b['trace']['host_thread_join_finalize_seconds']-a['trace']['host_thread_join_finalize_seconds'],launch_trace_output_and_exit_residue=b['launch_trace_output_and_exit_residue_seconds']-a['launch_trace_output_and_exit_residue_seconds']))
(P/'summary.json').write_text(json.dumps(dict(variants=summary,paired_median_seconds={k:statistics.median(x[k]for x in paired)for k in ['wall_delta','thread_start','thread_join_finalize','launch_trace_output_and_exit_residue']},paired_phase_median_seconds={k:statistics.median(x['phases'][k]for x in paired)for k in paired[0]['phases']},exact_outputs=18,diagnostic_only=True,limitations=['New diagnostic host executables preserve unchanged existing native Objects. Link layouts may differ from original executables.','Launch/trace-output/exit residue is not separately identified as process-exit time.','Median phase deltas are not additive; full per-round paired rows retained.','No trace timing qualifies a performance winner.']),indent=2)+'\n')
