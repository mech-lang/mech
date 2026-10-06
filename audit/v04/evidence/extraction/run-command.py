from pathlib import Path
import subprocess,time,json,os,hashlib,shutil,sys
root=Path(__file__).resolve().parents[4]; out=root/'audit/v04/evidence/extraction'
name=sys.argv[1]; cwd=Path(sys.argv[2]); cmd=sys.argv[3:]; env=os.environ.copy(); env.update(CARGO_TARGET_DIR=os.environ.get('MECH_EXTRACTION_TARGET_DIR', '/private/tmp/mech-v04-target-extraction/extracted'),CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0'); start=time.time()
with (out/(name+'.log')).open('w') as f: p=subprocess.run(cmd,env=env,cwd=cwd,stdout=f,stderr=subprocess.STDOUT)
r=dict(command=cmd,cwd=str(cwd),exit_code=p.returncode,elapsed_seconds=time.time()-start,environment={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','CARGO_INCREMENTAL']});(out/(name+'.json')).write_text(json.dumps(r,indent=2));print(json.dumps(r))
