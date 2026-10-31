"""Verify all archived versions, replay numbers without a compiler or compressor."""
from pathlib import Path
import hashlib,json,subprocess,sys,tarfile
packet=Path(__file__).resolve().parent;root=Path(sys.argv[1]);root.mkdir(parents=True,exist_ok=False)
records=sorted([(json.loads(p.read_text()),p.parent) for p in packet.glob('*/manifest.json')],key=lambda x:x[0]['created_utc']);occurrences=0;versions={}
assert records,'No archived receipts found'
for manifest,directory in records:
 with tarfile.open(directory/'receipts.tar.gz') as tar:
  by_name={m.name:m for m in tar.getmembers()};assert len(by_name)==len(manifest['files'])
  for item in manifest['files']:
   relative=Path(item['path']);assert not relative.is_absolute() and '..' not in relative.parts
   member=by_name[str(relative)];assert member.isfile();data=tar.extractfile(member).read()
   assert len(data)==item['bytes'] and hashlib.sha256(data).hexdigest()==item['sha256']
   target=root/relative;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(data);occurrences+=1;versions.setdefault(str(relative),set()).add(item['sha256'])
result={'archives':len(records),'verified_file_occurrences':occurrences,'unique_paths':len(versions),'paths_with_multiple_preserved_versions':sum(len(x)>1 for x in versions.values()),'scope':'Every archived version is hash-verified before chronological overlay. Numerical reconstruction uses archived raw observations/censuses and pins only; no compiler, compressor, executable or external scratch input.'}
for script,args in [('profile_analysis.py',[str(root),str(root/'profile-numbers.json')]),('allocation_analysis.py',[str(root),str(root/'allocation-comparison.json')]),('validation.py',[str(root),str(root/'validation.json')]),('analyze.py',[str(root),str(root/'replayed')])]:
 q=subprocess.run([sys.executable,str(packet/script),*args],capture_output=True,text=True);assert q.returncode==0,q.stdout+q.stderr
outputs=[('scorecard.json',root/'replayed/scorecard.json'),('scorecard.csv',root/'replayed/scorecard.csv'),('allocation-comparison.json',root/'allocation-comparison.json'),('profile-numbers.json',root/'profile-numbers.json'),('validation.json',root/'validation.json')]
for name,path in outputs:assert (packet/name).read_bytes()==path.read_bytes(),name
result['byte_identical_outputs']=[name for name,_ in outputs];print(json.dumps(result,indent=2))
