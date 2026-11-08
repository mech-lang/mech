from pathlib import Path
import json,subprocess
P=Path('/private/tmp/mech-related-borrowed-dictionary-values-emission-control-20261108-v3');O=Path('/private/tmp/mech-related-borrowed-dictionary-values-opt-20261108-v5');S='/private/tmp/mech-related-borrowed-dictionary-values-compiler-20261108'
b=json.loads((O/'build.json').read_text());assert all(x['exit']==0 for x in b['stages']);stage=b['stages'][-1];c=[a.replace(S,str(P))for a in stage['command']];c[-1]=str(P/'libmech_gpu.rlib')
for i,a in enumerate(c):
 if a.startswith('metadata='):c[i]='metadata=related_borrowed_dictionary_values_compact_emission_control_ir306_v3'
env={k:v.replace(S,str(P))for k,v in stage['environment'].items()};(P/'recipe.json').write_text(json.dumps(dict(command=c,environment=env,base_build=str(O/'build.json')),indent=2)+'\n')
for script in ['build-control.py','build-runner.py']:assert subprocess.run(['python3',str(P/script)]).returncode==0
