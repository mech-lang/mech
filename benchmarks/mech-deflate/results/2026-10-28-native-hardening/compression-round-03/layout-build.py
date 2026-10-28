from pathlib import Path
import json,subprocess,hashlib
root=Path(__file__).resolve().parent
build=json.loads((root/'current-build.json').read_text())
libs=build['libraries']
command=build['command'][:]
i=command.index('-C');command[i+1]='opt-level=1'
link=command.index('-C',i+2);assert command[link+1].startswith('link-arg=');del command[link:link+2]
command[-3:]=[str(root/'layout.rs'),'-o',str(root/'layout')]
assert not (root/'layout').exists()
r=subprocess.run(command,capture_output=True,text=True)
receipt={'command':command,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr}
(root/'layout-build.json').write_text(json.dumps(receipt,indent=2)+'\n');r.check_returncode()
r=subprocess.run([str(root/'layout')],capture_output=True,text=True)
(root/'layout-run.json').write_text(json.dumps({'command':[str(root/'layout')],'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr,'sha256':hashlib.sha256((root/'layout').read_bytes()).hexdigest()},indent=2)+'\n');r.check_returncode();print(r.stdout)
