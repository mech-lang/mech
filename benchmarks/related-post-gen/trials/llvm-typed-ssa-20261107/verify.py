import gzip,hashlib,importlib.util,json,pathlib,re
P=pathlib.Path(__file__).parent;m=json.loads((P/'manifest.json').read_text());read=lambda n:gzip.decompress((P/n).read_bytes())if n.endswith('.gz')else(P/n).read_bytes()
for n,v in m['files'].items():
 b=(P/n).read_bytes();assert len(b)==v['bytes']and hashlib.sha256(b).hexdigest()==v['sha256'],n
 d=gzip.decompress(b)if v['gzip']else b;assert len(d)==v['decoded_bytes']and hashlib.sha256(d).hexdigest()==v['decoded_sha256'],n
spec=importlib.util.spec_from_file_location('facts',P/'machine-reader/machine_facts.py');f=importlib.util.module_from_spec(spec);spec.loader.exec_module(f)
for name,count in [('after-4',55),('reuse-4',53)]:
 base='qualification/'+name+'/';names=json.loads(read(base+'definitions.json'));assert len(names)==count
 source=read(base+'native.ll.gz').decode();assert re.findall(r'^define .*? @"([^"\n]+)"\(',source,re.M)==names
 facts=f.extract(read(base+'native.o.gz'),read(base+'remarks.yaml').decode(),names);assert facts==json.loads(read(base+'machine-facts.json'))
 summary=json.loads(read('qualification/summary.json'));assert summary[name]['facts']==facts
assert '896 passed; 2 failed; 3 ignored' in summary['full_GPU_summary'][0]and summary['inherited_failures_exact']and summary['source_inventory_exact']
native=json.loads(read('qualification/native-semantics-tests.json'));assert native['exit']==0 and '3 passed; 0 failed' in native['stdout']
ci=json.loads(read('qualification/ci-selected-inventory-audit.json'));assert ci['expected']==ci['listed']==ci['executed_passed']==399 and ci['all_names_match']
executed=json.loads(read('qualification/ci-selected-execute.json'));passed=[l[len('test '):].split(' ... ')[0]for l in executed['stdout'].splitlines()if l.startswith('test ')and l.endswith(' ... ok')];assert sorted(passed)==sorted(ci['names'])
eq=json.loads(read('default-equivalence/equivalence.json'));assert len(eq)==2 and all(x['all13_bundle_files_byte_exact']and len(x['files'])==13 for x in eq.values())
assert not m['selected_changed']and not m['LLVM_program_qualified']and not m['performance_measured']
print(json.dumps(dict(files=len(m['files']),native_helper_tests=3,GPU_passed=896,inherited_failures=2,actual_CI_passed=399,original_definitions=55,reuse_definitions=53,default_bundle_files=26,LLVM_program_qualified=False,performance_measured=False)))
