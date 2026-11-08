from pathlib import Path
import hashlib,json,os,random,re,subprocess,time
P=Path(__file__).parent/'profile';INPUT=Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json');ORACLE=Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json')
read=lambda p:json.loads(Path(p).read_text())
def write(p,d):Path(p).write_text(json.dumps(d,indent=2)+'\n')
variants=[]
for x in read(P/'variants.json'):
 for detail in ([False,True]if x['kind']=='mech'else[False]):
  variants.append(dict(**x,variant=x['name']+('-detail'if detail else'-coarse'),helpers=detail,diagnostic=True))
for name,kind,exe,source in [('ordinary-mech','mech','/private/tmp/mech-related-borrowed-map-key-views-emission-control-20261108/histogram-reuse/runner-aot','/private/tmp/mech-related-borrowed-map-key-views-emission-control-20261108/histogram-reuse/build/source.mec'),('ordinary-histogram','mech','/private/tmp/mech-related-borrowed-map-key-views-runners-20261108-v1/histogram-reuse/runner-aot','/private/tmp/mech-related-borrowed-map-key-views-runners-20261108-v1/histogram-reuse/build/source.mec'),('ordinary-rust','rust','/private/tmp/mech-related-constant-key-rust256-controls-20261107-v1/default/target/release/rust_mech',None)]:variants.append(dict(name=name,variant=name,kind=kind,executable=exe,source=source,diagnostic=False,executable_sha256=hashlib.sha256(Path(exe).read_bytes()).hexdigest()))
assert len(variants)==8
protocol=dict(rounds=8,seed=99039021,variants=variants,diagnostic_only=True,promotion_timing_eligible=False,input_sha256=hashlib.sha256(INPUT.read_bytes()).hexdigest(),coarse_phases=['preparation_interning_and_eligibility','triple_index_and_coverage','quartet_index_and_masks','queries_cleanup_and_materialization'],process_instructions_all_workers=True,calling_thread_counters_for_nested_scopes=True,thread_and_global_allocator_counters=True,all_first_and_adverse_captures_retained=True,unchanged_native_Objects=True)
write(P/'capture-protocol.json',protocol)
snapshot=subprocess.check_output(['ps','-axo','pid,ppid,etime,pcpu,command'],text=True);busy=[]
for line in snapshot.splitlines()[1:]:
 a=line.split(None,4)
 if len(a)!=5 or int(a[0])in[os.getpid(),os.getppid()]:continue
 if re.search(r'(?:^|/)(?:rustc|cargo|clang|clang\+\+|llc|opt|sample|gzip|xz|zstd|gpu-tests|one-kib|scalar|runner-(?:aot|metrics|profile|pc|trace|phases))(?:\s|$)',a[4]):busy.append(a[:2]+[a[4].split()[0]])
write(P/'profiles-quiet.json',dict(no_other_own_build_test_profile_archive=not busy,busy=busy,process_snapshot_sha256=hashlib.sha256(snapshot.encode()).hexdigest()));assert not busy,busy
rng=random.Random(protocol['seed']);rows=[];expected=read(ORACLE);objects=P/'output-objects';objects.mkdir(exist_ok=True)
for pair in range(protocol['rounds']):
 order=variants[:];rng.shuffle(order)
 for position,x in enumerate(order):
  work=P/'captures'/f"{pair:02}-{x['variant']}";work.mkdir(parents=True,exist_ok=False);env={};cwd=work;output=work/'output.json'
  if x['kind']=='rust':
   cwd=work/'rust';cwd.mkdir();(work/'posts.json').symlink_to(INPUT);output=work/'related_posts_rust_mech.json';cmd=[x['executable']]
  else:cmd=[x['executable'],x['source'],str(INPUT),str(output),'1']
  if x['diagnostic']:env=dict(MECH_INTERVAL_COUNTERS='1',MECH_INTERVAL_HELPERS='1'if x['helpers']else'0',MECH_PHASE_BOUNDARIES=','.join(map(str,x['phase_boundaries'])),MECH_PHASE_TRACE=str(work/'trace.json'))
  t=time.monotonic();r=subprocess.run(cmd,cwd=cwd,env={**os.environ,**env},capture_output=True,text=True);row=dict(pair=pair,order=position,variant=x['variant'],kind=x['kind'],diagnostic=x['diagnostic'],command=cmd,cwd=str(cwd),environment=env,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,diagnostic_only=True,promotion_timing_eligible=False);write(work/'run.json',row);assert r.returncode==0,(x['variant'],r.returncode,r.stderr)
  b=output.read_bytes();sha=hashlib.sha256(b).hexdigest();blob=objects/(sha+'.json')
  if not blob.exists():assert json.loads(b)==expected;blob.write_bytes(b)
  else:assert b==blob.read_bytes()
  row.update(output_sha256=sha,output_object=str(blob),exact_Rust_output=True);output.unlink()
  if x['diagnostic']:
   trace=work/'trace.json'if x['kind']=='rust'else Path(str(output)+'.intervals.json');c=read(trace);j=c['capture'];e=j['events'];assert j['dropped']==j['unmatched_ends']==j['counter_errors']==0;assert j['coarse_checkpoints']==3;assert j['calling_thread_counters'];assert all(v['closed']and v['start_counters']and v['end_counters']for v in e);assert all(v['end_counters'][k]>=v['start_counters'][k]for v in e for k in ['instructions','cycles']);roots=[v for v in e if v['stage']=='post_JSON'];assert len(roots)==1;points=sorted([v for v in e if v['stage']=='coarse_checkpoint'],key=lambda v:v['operation']);assert [v['operation']for v in points]==[1,2,3];assert [v['site']for v in points]==x['phase_boundaries'];allpoints=[(roots[0],'start'),*[(v,'start')for v in points],(roots[0],'end')]
   for (a,sidea),(b,sideb)in zip(allpoints,allpoints[1:]):
    assert a[sidea+'_ns']<=b[sideb+'_ns'];assert all(a[sidea+'_process_counters'][k]<=b[sideb+'_process_counters'][k]for k in ['instructions','cycles'])
   threads={v['thread']for v in e};assert len(threads)==(5 if x['kind']=='mech' else 1);assert all(v['thread']==c['main_thread']or roots[0]['start_ns']<=v['start_ns']<=v['end_ns']<=roots[0]['end_ns']for v in e);row.update(trace=str(trace),events=len(e),threads=len(threads),native_body_events=sum(v['stage']=='native_body'for v in e),balanced_current_coarse_points=True)
  assert hashlib.sha256(Path(x['executable']).read_bytes()).hexdigest()==x['executable_sha256'];row['executable_sha256']=x['executable_sha256'];rows.append(row);write(P/'profiles.json',rows)
 print('round',pair+1,'of8: all8 exact',flush=True)
write(P/'capture-verification.json',dict(exact_outputs=len(rows),instrumented_captures=sum(r['diagnostic']for r in rows),ordinary_controls=sum(not r['diagnostic']for r in rows),all_coarse_points_balanced=True,all_outputs_qualified=True,all_executable_hashes_unchanged=True,diagnostic_only=True,promotion_timing_eligible=False))
