from pathlib import Path
import json,subprocess,time,os
out=Path(__file__).parent
build=Path('/private/tmp/mech-related-shared-call-certificates-opt-20261107-v1/build.json')
command=json.loads((out/'one-kib-run.initial-missing-fixtures.json').read_text())['command']
t=time.monotonic();r=subprocess.run(command,capture_output=True,text=True);(out/'one-kib-run.json').write_text(json.dumps(dict(command=command,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,compiler_build=str(build)),indent=2)+'\n');assert r.returncode==0,r.stdout+r.stderr
physical_fields={"allocation_requests_per_100","requested_bytes_per_100","allocation_requests","requested_bytes"}
for kind,n in [("scalar",128),("one-kib",5)]:
 def rows(x):return [{k:v for k,v in json.loads(line).items() if k not in ("seconds","seconds_per_evaluation")} for line in x["stdout"].splitlines() if line.startswith("{")]
 prior=Path("/private/tmp/mech-related-child-entry-facts-integration-regressions-20261107-v1")
 old=json.loads((prior/f"{kind}-run.json").read_text());new=json.loads((out/f"{kind}-run.json").read_text());a,b=rows(old),rows(new);assert len(a)==len(b)==n
 semantic=[];all_differences=[];physical=[]
 for i,(x,y) in enumerate(zip(a,b)):
  if x!=y:all_differences.append(dict(row=i,before=x,after=y))
  before={k:v for k,v in x.items() if k not in physical_fields};after={k:v for k,v in y.items() if k not in physical_fields}
  if before!=after:semantic.append(dict(row=i,before=before,after=after))
  physical.append(dict(row=i,metrics={k:dict(before=x[k],after=y[k],delta=y[k]-x[k])for k in physical_fields if k in x and k in y}))
 summary=dict(rows=n,all_nontiming_fields_equal_IR240=not all_differences,all_semantic_nontiming_fields_equal_IR240=not semantic,semantic_differences=semantic,all_field_differences=all_differences,physical_allocator_measurements=physical,physical_field_names=sorted(physical_fields),allocation_requests_never_regress=all(v["after"]<=v["before"]for row in physical for k,v in row["metrics"].items() if "requests"in k),requested_bytes_never_regress=all(v["after"]<=v["before"]for row in physical for k,v in row["metrics"].items() if "bytes"in k))
 (out/f"{kind}-comparison.json").write_text(json.dumps(summary,indent=2)+"\n")
 assert not semantic,json.dumps(semantic[:3])
 print(json.dumps({k:summary[k]for k in ["rows","all_nontiming_fields_equal_IR240","all_semantic_nontiming_fields_equal_IR240","allocation_requests_never_regress","requested_bytes_never_regress"]}),flush=True)
