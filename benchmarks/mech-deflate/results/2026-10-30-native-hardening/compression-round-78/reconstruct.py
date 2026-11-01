"""Hash-verify all versions and replay measurements without external inputs."""
from pathlib import Path
import hashlib,json,tarfile,subprocess,sys
packet=Path(__file__).resolve().parent;root=Path(sys.argv[1]);root.mkdir(parents=True,exist_ok=False);records=sorted([(json.loads(p.read_text()),p.parent) for p in packet.glob('*/manifest.json')],key=lambda v:v[0]['created_utc']);versions={};occurrences=0
for manifest,directory in records:
 with tarfile.open(directory/'receipts.tar.gz') as tar:
  members={m.name:m for m in tar.getmembers()};assert len(members)==len(manifest['files'])
  for item in manifest['files']:
   relative=Path(item['path']);assert not relative.is_absolute() and '..' not in relative.parts;member=members[str(relative)];assert member.isfile();raw=tar.extractfile(member).read();assert len(raw)==item['bytes'] and hashlib.sha256(raw).hexdigest()==item['sha256'];target=root/relative;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(raw);versions.setdefault(str(relative),set()).add(item['sha256']);occurrences+=1
for script,args in [('diagnostic_analysis.py',[str(root),str(root/'diagnostic-counts/summary.json')]),('profile_analysis.py',[str(root),str(root/'profile-numbers.json')]),('allocation_analysis.py',[str(root),str(root/'allocation-comparison.json')]),('instruction_analysis.py',[str(root),str(root/'instruction-window-comparison.json')]),('callback_analysis.py',[str(root),str(root/'callback-comparison.json')]),('validation.py',[str(root),str(root/'validation.json')]),('analyze.py',[str(root),str(root/'replayed')])]:
 q=subprocess.run([sys.executable,str(packet/script),*args],capture_output=True,text=True);assert q.returncode==0,q.stdout+q.stderr
outputs=[('analysis/scorecard.json',root/'replayed/scorecard.json'),('analysis/scorecard.csv',root/'replayed/scorecard.csv'),('diagnostic-counts/summary.json',root/'diagnostic-counts/summary.json'),('profile-numbers.json',root/'profile-numbers.json'),('allocation-comparison.json',root/'allocation-comparison.json'),('instruction-window-comparison.json',root/'instruction-window-comparison.json'),('callback-comparison.json',root/'callback-comparison.json'),('validation.json',root/'validation.json')]
for name,path in outputs:assert (packet/name).read_bytes()==path.read_bytes(),name
print(json.dumps({'archives':len(records),'verified_file_occurrences':occurrences,'unique_paths':len(versions),'paths_with_multiple_preserved_versions':sum(len(v)>1 for v in versions.values()),'byte_identical_outputs':[name for name,_ in outputs],'scope':'All archived versions verified before chronological overlay. Analysis-only replay; no compiler/compressor or external scratch files.'},indent=2))
