"""Isolated LuaJIT pi-digits study using the unchanged shared durable sampler helpers."""
import argparse
import fcntl
import importlib.util
import json
import subprocess
import sys
from pathlib import Path
HERE=Path(__file__).resolve().parent
C=HERE.parents[1]
N=C/'r-apl-uiua-comparison/20261011'
HELPER=N/'uiua-mandel-parallel-v4/full_study.py'
spec=importlib.util.spec_from_file_location('luajit_durable_study_helpers',HELPER)
helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
helper.HERE=HERE;helper.N=N;helper.OUT=HERE/'full-study'
helper.MANIFEST=helper.OUT/'manifest.json';helper.REPORT=HERE/'results/full-study.json'

def main():
 p=argparse.ArgumentParser();mode=p.add_mutually_exclusive_group(required=True);mode.add_argument('--run',action='store_true');mode.add_argument('--detach',action='store_true');p.add_argument('--manifest-sha256',required=True);args=p.parse_args()
 if args.run:helper.run(args.manifest_sha256);return
 with helper.LOCK.open('a+') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX)
  assert helper.sha(helper.MANIFEST)==args.manifest_sha256,'Reviewed manifest changed'
  manifest=json.loads(helper.MANIFEST.read_text());assert manifest['status']=='prepared-awaiting-root-review'
  helper.load_harness().PinGuard(manifest['pins']).verify()
  with (helper.OUT/'launch-owner.lock').open('a+') as launch_lock:
   fcntl.flock(launch_lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
   assert not (helper.OUT/'launch.json').exists() and not helper.REPORT.exists(),'Never launch duplicate or silently resume an existing study'
   with (helper.OUT/'supervisor.log').open('ab') as log:
    process=subprocess.Popen([sys.executable,'-u',str(Path(__file__).resolve()),'--run','--manifest-sha256',args.manifest_sha256],stdin=subprocess.DEVNULL,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
   receipt=dict(pid=process.pid,identity=helper.identity(process.pid),manifest_sha256=args.manifest_sha256,launched_at=helper.now())
   helper.save(helper.OUT/'launch.json',receipt);print(json.dumps(receipt,indent=2),flush=True)
if __name__=='__main__':main()
