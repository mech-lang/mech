from pathlib import Path
import json,os,subprocess,time,hashlib
main=Path("/Users/cim310/mech-lang/mech")
out=Path(__file__).parent
build=Path("/private/tmp/mech-related-owned-shape-metadata-opt-20261107-v1/build.json")
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
allowed=set()
for kind,n in [("scalar",128),("one-kib",5)]:
 def rows(x):return [{k:v for k,v in json.loads(line).items() if k not in ("seconds","seconds_per_evaluation")} for line in x["stdout"].splitlines() if line.startswith("{")]
 old=json.loads((Path("/private/tmp/mech-related-local-facts-integration-regressions-20261107-v1")/f"{kind}-run.json").read_text());new=json.loads((out/f"{kind}-run.json").read_text());a,b=rows(old),rows(new);assert len(a)==len(b)==n
 semantic=[];resources=[]
 for i,(x,y) in enumerate(zip(a,b)):
  ax={k:v for k,v in x.items()if k not in allowed};by={k:v for k,v in y.items()if k not in allowed}
  if ax!=by:semantic.append(dict(row=i,before=ax,after=by))
  metrics={k:dict(before=x[k],after=y[k],delta=y[k]-x[k]) for k in allowed if k in x and k in y}
  assert all(v['after']<=v['before'] for v in metrics.values()),metrics
  resources.append(dict(row=i,metrics=metrics))
 summary=dict(rows=n,all_nontiming_fields_equal_IR236=not semantic,allocation_requests_and_bytes_never_regress=True,allocation_changes=resources,differences=semantic)
 (out/f"{kind}-comparison.json").write_text(json.dumps(summary,indent=2)+"\n");assert not semantic,json.dumps(semantic[:3]);print(json.dumps(dict(kind=kind,rows=n,all_nontiming_fields_equal_IR236=True,allocation_requests_and_bytes_never_regress=True)),flush=True)
