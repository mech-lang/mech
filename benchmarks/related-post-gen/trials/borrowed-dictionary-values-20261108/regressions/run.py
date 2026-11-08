from pathlib import Path
import json,os,subprocess,time,hashlib
main=Path("/Users/cim310/mech-lang/mech")
out=Path(__file__).parent
build=Path("/private/tmp/mech-related-borrowed-dictionary-values-opt-20261108-v4/build.json")
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
 r=run(cmd,out/f"{kind}.build.json");assert (out/kind).stat().st_mode & 0o111, "compiler produced a non-executable output";assert (out/kind).read_bytes()[:4]==bytes.fromhex("cffaedfe"), "compiler output is not a complete Mach-O";assert not ("stripping debug info" in r["stderr"] and "failed" in r["stderr"]), r["stderr"];r["executable_sha256"]=hashlib.sha256((out/kind).read_bytes()).hexdigest();(out/f"{kind}.build.json").write_text(json.dumps(r,indent=2)+"\n")
 cmd=json.loads((base/f"numeric-workers-v8-{kind}-run.json").read_text())["command"];cmd[0]=str(out/kind)
 r=run(cmd,out/f"{kind}-run.json")
 print(json.dumps(dict(kind=kind,executed=True)),flush=True)

import re
physical_fields={"allocation_requests_per_100","requested_bytes_per_100","allocation_requests","requested_bytes"}
base=Path('/private/tmp/mech-related-native-dictionary-seed-frames-regressions-20261108-v1')
def rows(f):return [{k:v for k,v in json.loads(l).items()if k not in ['seconds','seconds_per_evaluation']}for l in json.loads(f.read_text())['stdout'].splitlines()if l.startswith('{')]
def report(s):return {k:int(v)for k,v in re.findall(r'(\w+): (\d+)',s)}
for kind,n in [('scalar',128),('one-kib',5)]:
 a,b=rows(base/(kind+'-run.json')),rows(out/(kind+'-run.json'));assert len(a)==len(b)==n;changes=[]
 for i,(x,y)in enumerate(zip(a,b)):
  assert {k:v for k,v in x.items()if k not in physical_fields|{'report'}}=={k:v for k,v in y.items()if k not in physical_fields|{'report'}}
  pd={k:dict(before=x[k],after=y[k],delta=y[k]-x[k])for k in physical_fields if k in x}
  rd={}
  if 'report'in x:
   rx,ry=report(x['report']),report(y['report']);assert rx.keys()==ry.keys();rd={k:dict(before=rx[k],after=ry[k],delta=ry[k]-rx[k])for k in rx if rx[k]!=ry[k]};assert set(rd)<={'work','visits'}
  changes.append(dict(row=i,physical=pd,report_resource_changes=rd))
 summary=dict(rows=n,all_output_and_nonresource_fields_equal_IR305=True,all_allocation_counts_and_requested_bytes_nonincreasing=all(v["delta"]<=0 for r in changes for v in r["physical"].values()),allowed_changed_report_fields=['work','visits'],report_work_and_visits_only_decrease=all(v["delta"]<=0 for r in changes for v in r["report_resource_changes"].values()),changes=changes,pricing_note='IR306 adds borrowed keys and independent numeric cells with complete original admission. All output and nonresource fields must match IR305. Every physical allocation/requested-byte and work/visit difference remains explicit; no resource improvement is assumed. Descriptor prices and finite execution contracts are checked separately.')
 (out/(kind+'-comparison.json')).write_text(json.dumps(summary,indent=2)+'\n');print(kind,n,'exact outputs/nonresource fields; resource/report differences retained',flush=True)
