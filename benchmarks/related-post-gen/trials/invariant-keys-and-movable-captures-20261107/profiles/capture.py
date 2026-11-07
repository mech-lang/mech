from pathlib import Path
from datetime import datetime,timezone
import subprocess,time,json,hashlib
P=Path(__file__).parent;INPUT=Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json');ORACLE=Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json');sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
raw=subprocess.check_output(['ps','-axo','pid,ppid,etime,pcpu,command'],text=True);needles=['rustc --crate-name','cargo build','cargo test','/Users/cim310/builds/','build-metrics.py','mech_gpu-tests','all-thread-profile-build-20261107.py','compare-implementations.py'];a=[s for s in raw.splitlines()if any(k in s for k in needles)and 'capture.py' not in s];assert not a,a
(P/'quiet-start-gate.json').write_text(json.dumps(dict(quiet=True,process_snapshot=raw,profile_timings_excluded=True,scope='All-thread diagnostic of repeated full evaluations including driver input cloning and verification, outside the eligible latency cohorts.'),indent=2)+'\n')
for name in ['selected-ir242','candidate-ir248']:
 p=P/name;cmd=[str(p/'runner-profile'),str(p/'build/source.mec'),str(INPUT),str(p/'output.json'),'1500'];start=time.monotonic()
 with (p/'stdout.log').open('w')as out,(p/'stderr.log').open('w')as err:
  process=subprocess.Popen(cmd,stdout=out,stderr=err)
  sample_cmd=['/usr/bin/sample',str(process.pid),'8','1','-mayDie','-fullPaths','-file',str(p/'sample.txt')]
  sample_start=time.monotonic();sample=subprocess.run(sample_cmd,capture_output=True,text=True,timeout=40)
  (p/'sample-run.json').write_text(json.dumps(dict(command=sample_cmd,pid=process.pid,exit=sample.returncode,seconds=time.monotonic()-sample_start,stdout=sample.stdout,stderr=sample.stderr),indent=2)+'\n')
  result=process.wait(timeout=60)
 rows=[json.loads(s)for s in(p/'stdout.log').read_text().splitlines()if s.startswith('{')];exact=result==0 and len(rows)==1500 and json.loads((p/'output.json').read_text())==json.loads(ORACLE.read_text());assert exact,(name,result,(p/'stderr.log').read_text())
 report=dict(created_utc=datetime.now(timezone.utc).isoformat(),command=cmd,pid=process.pid,exit=result,diagnostic_wall_seconds=time.monotonic()-start,iterations=len(rows),every_iteration_indices_checked=True,first_last_full_json_checked=True,final_json_exact_Rust=exact,input_sha256=sha(INPUT),oracle_sha256=sha(ORACLE),source_sha256=sha(p/'build/source.mec'),executable_sha256=sha(p/'runner-profile'),sample_exit=sample.returncode,sample_file_exists=(p/'sample.txt').exists(),native_link_audit='passed',profile_timings_excluded=True,scope='All live threads sampled by macOS sample during repeated full evaluations. Driver cloning and verification are included; descriptor is restored once. This is not the continuous post-JSON scope or a latency/resource cohort. Sample call-tree counts are not precise CPU-time percentages.')
 (p/'run.json').write_text(json.dumps(report,indent=2)+'\n');print(name,'1500 exact evaluations, sample exit',sample.returncode,'profile bytes',(p/'sample.txt').stat().st_size if(p/'sample.txt').exists()else 0,flush=True)
