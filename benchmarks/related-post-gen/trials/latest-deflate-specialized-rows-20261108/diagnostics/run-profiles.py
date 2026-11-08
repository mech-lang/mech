from pathlib import Path
import hashlib,json,os,random,re,subprocess,time
P=Path(__file__).parent
INPUT=Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json')
ORACLE=Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json')
lat=json.loads(Path('/private/tmp/mech-related-latest-deflate-20261108/latency-config.json').read_text());v={x['name']:x for x in lat['implementations']}
variants=[]
for label,name in [('plain','mech-ir280-latest-deflate-4'),('both-rows','mech-ir280-both-rows-4')]:
 x=v[name];variants.append(dict(name='ordinary-'+label,kind='ordinary',executable=x['executable'],source=x['source']))
 for detail in [False,True]:variants.append(dict(name='profile-'+label+('-callbacks'if detail else'-native'),kind='profile',executable=str(P/label/'runner-profile'),source=str(P/label/'build/source.mec'),helpers=detail))
assert len(variants)==6
pins={x['executable']:hashlib.sha256(Path(x['executable']).read_bytes()).hexdigest()for x in variants}
freeze=dict(diagnostic_only=True,promotion_timing_eligible=False,seed=99029410,rounds=8,variants=variants,executable_pins=pins,input_sha256=hashlib.sha256(INPUT.read_bytes()).hexdigest(),all_first_and_adverse_samples_retained=True,method='Calling-thread snapshots for instrumented runtime with ordinary production controls; identical instrumented runtime for both unchanged native Objects. No source algorithm change.')
(P/'capture-protocol.json').write_text(json.dumps(freeze,indent=2)+'\n')
snapshot=subprocess.check_output(['ps','-axo','pid,ppid,etime,pcpu,command'],text=True);busy=[]
for line in snapshot.splitlines()[1:]:
 a=line.split(None,4)
 if len(a)!=5 or int(a[0])in[os.getpid(),os.getppid()]:continue
 if re.search(r'(?:^|/)(?:rustc|cargo|clang|clang\+\+|llc|opt|sample|gzip|xz|zstd|gpu-tests|one-kib|scalar|runner-(?:aot|metrics|profile|pc|trace|phases))(?:\s|$)',a[4]):busy.append(a[:2]+[a[4].split()[0]])
assert not busy,busy
(P/'profiles-quiet.json').write_text(json.dumps(dict(no_other_own_build_test_profile_archive=True,process_snapshot_sha256=hashlib.sha256(snapshot.encode()).hexdigest()),indent=2)+'\n')
objects=P/'output-objects';objects.mkdir(exist_ok=True);rng=random.Random(freeze['seed']);rows=[];expected=json.loads(ORACLE.read_text())
for pair in range(8):
 order=variants[:];rng.shuffle(order)
 for order_index,x in enumerate(order):
  work=P/'captures'/f"{pair:02}-{x['name']}";work.mkdir(parents=True,exist_ok=True)
  output=work/'output.json';cwd=work;env={};trace=None
  if x['kind']=='exact':
   env={'DYLD_INSERT_LIBRARIES':str(P/'exact-lifecycle.dylib'),'MECH_EXACT_OUTPUT':str(output),'MECH_EXACT_TRACE':str(work/'trace.json')}
  if x.get('upstream'):
   cwd=work/'rust';cwd.mkdir(exist_ok=True);(work/'posts.json').symlink_to(INPUT);output=work/'related_posts_rust_mech.json';env['MECH_EXACT_OUTPUT']='../related_posts_rust_mech.json';cmd=[x['executable']]
  else:cmd=[x['executable'],x['source'],str(INPUT),str(output),'1']
  if x['kind']=='profile':env.update(MECH_INTERVAL_COUNTERS='1',MECH_INTERVAL_HELPERS='1'if x['helpers']else'0')
  t=time.monotonic();r=subprocess.run(cmd,cwd=cwd,capture_output=True,text=True,env={**os.environ,**env});elapsed=time.monotonic()-t
  row=dict(pair=pair,order=order_index,variant=x['name'],kind=x['kind'],command=cmd,cwd=str(cwd),environment=env,exit=r.returncode,seconds=elapsed,stdout=r.stdout,stderr=r.stderr,diagnostic_only=True,promotion_timing_eligible=False)
  (work/'run.json').write_text(json.dumps(row,indent=2)+'\n');assert r.returncode==0,(x['name'],r.returncode,r.stderr)
  b=output.read_bytes();key=hashlib.sha256(b).hexdigest();blob=objects/(key+'.json')
  if not blob.exists():assert json.loads(b)==expected;blob.write_bytes(b)
  else:assert b==blob.read_bytes()
  row.update(output_sha256=key,exact_Rust_output=True,output_object=str(blob));output.unlink()
  if x['kind']=='profile':
   trace=Path(str(output)+'.intervals.json');j=json.loads(trace.read_text());c=j['capture'];events=c['events'];assert c['dropped']==c['unmatched_ends']==c['counter_errors']==0;assert c['calling_thread_counters'];assert all(e['closed']and e['start_counters']and e['end_counters']for e in events);assert all(e['end_ns']>=e['start_ns']and e['end_counters']['instructions']>=e['start_counters']['instructions']and e['end_counters']['cycles']>=e['start_counters']['cycles']for e in events);assert sum(e['stage']=='native_body'for e in events)==23;assert len({e['thread']for e in events})==5;assert sum(e['stage']=='driver_total'for e in events)==1
   row.update(trace=str(trace),events=len(events),native_intervals=23,threads=5,all_counters_monotonic=True)
  elif x['kind']=='exact':
   trace=work/'trace.json';assert trace.exists(),x['name'];j=json.loads(trace.read_text());assert j['exact_executable_not_recompiled']and j['target_open_count']==1 and j['closed']and j['close_result']==0;points=[j[k]for k in ['opened','before_close','after_close','thread_exit']];assert all(p['error']==0 for p in points);assert all(all(a[k]<=b[k]for k in ['instructions','cycles','ticks'])for a,b in zip(points,points[1:]));row['trace']=str(trace)
  row['executable_sha256']=pins[x['executable']];rows.append(row);(P/'profiles.json').write_text(json.dumps(rows,indent=2)+'\n')
 print('round',pair+1,'of8: all6 exact',flush=True)
for exe,sha in pins.items():assert hashlib.sha256(Path(exe).read_bytes()).hexdigest()==sha
assert len(rows)==48
(P/'capture-verification.json').write_text(json.dumps(dict(exact_outputs=48,instrumented_captures=32,exact_binary_lifecycle_captures=0,ordinary_controls=16,all_5_thread_profiles=True,no_counter_or_loan_errors=True,all_executable_hashes_unchanged=True,diagnostic_only=True),indent=2)+'\n')
