from pathlib import Path
import gzip,hashlib,json,re,statistics,random,math
P=Path(__file__).parent
manifest=json.loads((P/'manifest.json').read_text());records={r['path']:r for r in manifest['records']}
def data(name):
 p=P/name
 b=p.read_bytes()
 return gzip.decompress(b)if name.endswith('.gz')else b
def load(name):return json.loads(data(name))
def find(name):
 if name in records:return name
 assert name+'.gz'in records,name
 return name+'.gz'
for name,r in records.items():
 assert hashlib.sha256((P/name).read_bytes()).hexdigest()==r['sha256'],name
 assert hashlib.sha256(data(name)).hexdigest()==r['decoded_sha256'],name
 assert len(data(name))==r['decoded_bytes'],name
lat=load(find('results/comparison-native-recovered-parent-metadata-20261108-5000-m3.json'));res=load(find('results/comparison-native-recovered-parent-metadata-resources-20261108-5000-m3.json'));a=load('current/paired-analysis.json');rule=load('current/prospective-rule.json');mapping=load('output-map.json');oracle=load(find('input/oracle.json'));outputs={}
assert len(lat['samples'])==930 and len(res['samples'])==217 and len(mapping)==1147
assert hashlib.sha256(data(find('input/posts.json'))).hexdigest()=='500530b81cb81c5b1a5e48aa30de6a89ba0a350460ad826c4c7bb023e6e46cb8'
rawmap={(r['cohort'],r['variant'],r['sample']):r for r in mapping};assert len(rawmap)==1147
for label,report in [('latency',lat),('resources',res)]:
 for row in report['samples']:
  m=rawmap[(label,row['variant'],row['sample'])];raw=data(m['retained_output'])
  assert row['exit']==0 and row['exact_rust_output']
  assert hashlib.sha256(raw).hexdigest()==row['output_sha256']==m['output_sha256']
  if row['output_sha256']not in outputs:outputs[row['output_sha256']]=json.loads(raw)
  assert outputs[row['output_sha256']]==oracle
  if 'measurement'in row:
   rawjson=next((json.loads(s.removeprefix('PROCESS_METRICS '))for s in row['stdout'].splitlines()if s.startswith('PROCESS_METRICS ')or s.startswith('{')),None)
   if 'measurement'in rawjson:assert rawjson['measurement']==row['measurement']
  if label=='resources':
   raw=data(m['raw_process_resources']).decode();parsed={}
   for line in raw.splitlines():
    g=re.fullmatch(r'\s*(\d+)\s+(.+?)\s*',line)
    if g:
     key=g[2].replace(' ','_');key={'maximum_resident_set_size':'peak_resident_bytes','peak_memory_footprint':'peak_physical_footprint_bytes','instructions_retired':'instructions_retired','cycles_elapsed':'cycles'}.get(key,key);parsed[key]=int(g[1])
   assert all(row['process_resources'][k]==v for k,v in parsed.items())
for name,m in a['medians'].items():
 rows=[x for x in lat['samples']if x['variant']==name];assert len(rows)==31
 for k in ['post_json_seconds','wall_seconds','compile_or_restore_seconds']:assert statistics.median(x[k]for x in rows)==m[k]
 assert m['post_json_seconds']/a['fastest_processing_Rust']['median_seconds']==m['times_fastest_processing_Rust']
 rr=[x for x in res['samples']if x['variant']==name+'-instrumented'];assert len(rr)==7
 for key,field in [('restore','restore_measurement'),('post_JSON','measurement')]:
  for metric,v in a['resource_medians'][name][key].items():assert statistics.median(x[field]['hardware'if metric in ['instructions_retired','cycles']else metric][metric]if metric in ['instructions_retired','cycles']else x[field][metric]for x in rr)==v
eligible=a['fastest_processing_Rust']['eligible_variants'];med=lambda n:statistics.median(x.get('post_json_seconds',x['seconds'])for x in lat['samples']if x['variant']==n)
assert min(eligible,key=med)==a['fastest_processing_Rust']['variant']
assert med(a['fastest_processing_Rust']['variant'])==a['fastest_processing_Rust']['median_seconds']
for name,d in a['decisions'].items():
 parent=d['matched_parent']
 for boundary,pair in d['paired'][parent].items():
  values={n:{x['sample']:x[boundary]for x in lat['samples']if x['variant']==n}for n in [name,parent]};changes=[values[name][i]/values[parent][i]-1 for i in range(31)]
  assert changes==pair['all_pair_relative_changes']and sum(x<0 for x in changes)==pair['wins']and statistics.median(changes)==pair['median_paired_relative_change']
  rng=random.Random(rule['bootstrap']['seed']);b=sorted(statistics.median(changes[rng.randrange(31)]for _ in range(31))for _ in range(rule['bootstrap']['resamples']))
  def q(p):i=(len(b)-1)*p;lo=math.floor(i);hi=math.ceil(i);return b[lo]+(b[hi]-b[lo])*(i-lo)
  assert[q(.025),q(.975)]==pair['two_sided_95_interval']
listed=load(find('qualification/native/ci-list.json'));ran=load(find('qualification/native/ci-execute.json'));old=load(find('qualification/native/retained415-ci-list.json'))
getlist=lambda x:set(re.findall(r'^(.+): test$',x['stdout'],re.M));names=getlist(listed);executed=set(re.findall(r'^test (.+) \.\.\. ok$',ran['stdout'],re.M))
assert len(names)==418 and names==executed and getlist(old)<=names and len(getlist(old))==415 and listed['exit']==ran['exit']==0
rows=lambda x:[{k:v for k,v in json.loads(s).items()if k not in ['seconds','seconds_per_evaluation']}for s in x['stdout'].splitlines()if s.startswith('{')]
for kind,count in [('scalar',128),('one-kib',5)]:
 before=load(find('qualification/regressions/IR240-'+kind+'-run.json'));after=load(find('qualification/regressions/'+kind+'-run.json'));assert after['exit']==0 and rows(before)==rows(after)and len(rows(after))==count
for key,expected in [('plain',7919355),('both-rows',8006235)]:
 lines=[json.loads(s)for s in data(find('allocation-profile/'+key+'/allocations.jsonl')).decode().splitlines()];header=lines[0];events=lines[1:];summary=load('allocation-profile/'+key+'/summary.json')
 assert len(events)==2945==header['recorded']==header['requests']and header['buffer_dropped']==header['reentrant_dropped']==0
 assert sum(x['bytes']for x in events)==expected==summary['requested_bytes']and summary['exact_standard_rust_output']and summary['counts_and_bytes_equal_candidate_benchmark_resource_counters']
assert sum(x['requests']for x in load('allocation-profile/bound-call-clone-callers.json'))==381
# Independently compare complete emitted arithmetic bytes in both Mach-O Objects.
import importlib.util
spec=importlib.util.spec_from_file_location('macho_audit',P/'audit-native-link.py');ma=importlib.util.module_from_spec(spec);spec.loader.exec_module(ma)
for key in ['plain','both-rows']:
    before=data(find('objects/IR280-'+key+'/object.o'));after=data(find('objects/'+key+'/object.o'))
    def text_bytes(raw):
        import struct
        ncmds=struct.unpack_from('<I',raw,16)[0];offset=32
        for _ in range(ncmds):
            command,length=struct.unpack_from('<II',raw,offset)
            if command==0x19:
                sections=struct.unpack_from('<I',raw,offset+64)[0]
                for i in range(sections):
                    start=offset+72+i*80;name=raw[start:start+16].split(b'\0')[0]
                    if name==b'__text':
                        size=struct.unpack_from('<Q',raw,start+40)[0];position=struct.unpack_from('<I',raw,start+48)[0];return raw[position:position+size]
            offset+=length
        raise AssertionError('Missing __text')
    assert text_bytes(before)==text_bytes(after)
# The memory improvement is exact in every resource repetition.
for name,expected in [('mech-ir280-both-rows-4',(27302,13193328)),('mech-ir282-both-rows-borrowed-parent-4',(25607,13131832))]:
    rr=[x for x in res['samples']if x['variant']==name+'-instrumented'];assert len(rr)==7
    assert all((x['restore_measurement']['allocation_requests'],x['restore_measurement']['requested_allocation_bytes'])==expected for x in rr)
    assert all((x['measurement']['allocation_requests'],x['measurement']['requested_allocation_bytes'])==(2945,8006235)for x in rr)
receipt=dict(records_hashed=len(records),actual_cohort_outputs_rederived=1147,native_CI_names_rederived=418,retained_names=415,exact_scalar_DEFLATE_rows_rederived=133,current_allocation_events_rederived=5890,paired_current_parent_bootstrap_checks=6,all_passed=True)
(P/'verification-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt))
