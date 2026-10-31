from pathlib import Path
import hashlib,json,subprocess,sys,tarfile
packet=Path(__file__).resolve().parent;destination=Path(sys.argv[1]).resolve();destination.mkdir(parents=True,exist_ok=True);assert not any(destination.iterdir()),'use a fresh empty directory'
archives=[]
for manifest in packet.glob('*/manifest.json'):
 d=json.loads(manifest.read_text());archives.append((d['created_utc'],manifest.parent,d))
versions={};occurrences=0
for _,folder,manifest in sorted(archives):
 expected={x['path']:x for x in manifest['files']}
 with tarfile.open(folder/'receipts.tar.gz','r:gz') as t:
  members={x.name:x for x in t.getmembers()};assert set(members)==set(expected),(folder,set(members)^set(expected))
  for name,pin in expected.items():
   path=Path(name);assert not path.is_absolute() and '..' not in path.parts;member=members[name];assert member.isfile();data=t.extractfile(member).read();assert len(data)==pin['bytes'] and hashlib.sha256(data).hexdigest()==pin['sha256'];out=destination/path
   if out.exists() and out.read_bytes()!=data:
    prior=out.read_bytes();digest=hashlib.sha256(prior).hexdigest();v=destination/'versions'/digest/path;v.parent.mkdir(parents=True,exist_ok=True);v.write_bytes(prior)
   out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(data);versions.setdefault(name,set()).add(pin['sha256']);occurrences+=1
q=subprocess.run(['python3',str(destination/'analyze.py'),str(destination)],capture_output=True,text=True);q.check_returncode()
for name in ['metrics.json','metrics.csv','profile.json']:assert (destination/name).read_bytes()==(packet/name).read_bytes(),name
object_archive=packet.parent/'compression-round-51/compiled-artifact/receipts.tar.gz';q=subprocess.run(['python3',str(destination/'graph-path.py'),str(destination/'regenerated-graph-path.json'),str(object_archive)],capture_output=True,text=True);q.check_returncode();assert (destination/'regenerated-graph-path.json').read_bytes()==(packet/'graph-path.json').read_bytes()
print(json.dumps({'archives':len(archives),'file_occurrences':occurrences,'unique_paths':len(versions),'multi_version_paths':sum(len(v)>1 for v in versions.values()),'preserved_prior_versions':True,'numerical_json_csv_profile_byte_equal':True,'graph_witness_byte_equal':True,'benchmarks_or_compilers_run':False},indent=2))
