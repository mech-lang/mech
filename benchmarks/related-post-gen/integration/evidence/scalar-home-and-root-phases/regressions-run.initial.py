from pathlib import Path
import json,os,subprocess,time,hashlib
main=Path("/Users/cim310/mech-lang/mech")
out=Path(__file__).parent
build=Path("/private/tmp/mech-related-scalar-home-retirement-opt-20261107-v2/build.json")
libs={s["crate"]:s["command"][-1] for s in json.loads(build.read_text())["stages"]}
base=main/"benchmarks/related-post-gen/results/compiler-regressions"
def run(cmd,path):
 t=time.monotonic();r=subprocess.run(cmd,cwd=main,env={**os.environ,"RUSTC_BOOTSTRAP":"1"},capture_output=True,text=True);v=dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,compiler_build=str(build));path.write_text(json.dumps(v,indent=2)+"\n");assert r.returncode==0,r.stderr;return v
for kind in ("scalar","one-kib"):
 cmd=json.loads((base/f"numeric-workers-v8-{kind}.build.json").read_text())["command"]
 for i,a in enumerate(cmd):
  if a=="--extern":
   name=cmd[i+1].split("=",1)[0]
   if name in libs:cmd[i+1]=name+"="+libs[name]
  if a=="-L" and "mech-related-numeric-workers-opt-" in cmd[i+1]:cmd[i+1]="dependency="+str(build.parent)
 for parent in sorted({str(Path(path).parent) for path in libs.values()}):
  cmd.extend(["-L","dependency="+parent])
 cmd[cmd.index("-o")+1]=str(out/kind)
 r=run(cmd,out/f"{kind}.build.json");r["executable_sha256"]=hashlib.sha256((out/kind).read_bytes()).hexdigest();(out/f"{kind}.build.json").write_text(json.dumps(r,indent=2)+"\n")
 cmd=json.loads((base/f"numeric-workers-v8-{kind}-run.json").read_text())["command"];cmd[0]=str(out/kind)
 r=run(cmd,out/f"{kind}-run.json")
 print(json.dumps(dict(kind=kind,executed=True)),flush=True)
physical_fields={"allocation_requests_per_100","requested_bytes_per_100","allocation_requests","requested_bytes"}
for kind,n in [("scalar",128),("one-kib",5)]:
 def rows(x):return [{k:v for k,v in json.loads(line).items() if k not in ("seconds","seconds_per_evaluation")} for line in x["stdout"].splitlines() if line.startswith("{")]
 prior=Path("/private/tmp/mech-related-canonical-schema-sinks-regressions-20261107-v1")
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
