from pathlib import Path
import hashlib,importlib.util,json

R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
base=Path('/private/tmp/mech-related-borrowed-parent-frame-metadata-cohort-20261107-v2')
P=Path('/private/tmp/mech-related-boolean-loops-and-parent-counts-cohort-20261107-v2')
read=lambda p:json.loads(p.read_text())
resources=read(base/'resources-config.json')
for epoch,version in [(255,'v1'),(257,'v2')]:
 M=Path('/private/tmp/mech-related-boolean-loops-and-parent-counts-metrics-20261107-'+version)
 v=next(v for v in read(M/'metrics-config.json')['implementations']if v['name']==f'mech-ir{epoch}-reuse-4-instrumented')
 resources['implementations'].append(v)
assert len(resources['implementations'])==11
resources['authoritative_latency_comparison']=str(R/'results/comparison-native-boolean-loops-and-parent-counts-randomized-20261107-5000-m3.json')
(P/'resources-config.json').write_text(json.dumps(resources,indent=2)+'\n')
spec=importlib.util.spec_from_file_location('audit',R/'audit-native-link.py');audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)
def native_text(p):
 data,sections,_=audit.macho(p);s=next(s for s in sections if s['name']=='__text');return data[s['offset']:s['offset']+s['size']]
def function(p,header):
 meta=read(p/'metadata.json');f=next(f for f in meta['compiled_numeric_functions']if f['header']==header)
 data,sections,symbols=audit.macho(p/'object.o');symbol=next(s for s in symbols if s['name']=='_'+f['symbol']);s=sections[symbol['section']-1]
 at=s['offset']+symbol['value']-s['address'];return data[at:at+f['code_bytes']]
proof=[]
U252=Path('/private/tmp/mech-related-selected-range-integration-runners-20261107-v1')
U255=Path('/private/tmp/mech-related-parent-frame-runtime-counts-runners-20261107-v1')
U257=Path('/private/tmp/mech-related-boolean-loop-integration-runners-20261107-v3')
for variant in ['after-4','reuse-4']:
 x=native_text(U252/variant/'build/object.o');y=native_text(U255/variant/'build/object.o');assert x==y
 proof.append(dict(variant=variant,IR255_native_text_equals_IR252=True,native_text_bytes=len(y),sha256=hashlib.sha256(y).hexdigest()))
x=function(U252/'reuse-4/build',1282);y=function(U257/'reuse-4/build',1282);assert x==y
proof.append(dict(variant='reuse-4',header=1282,IR257_hot_query_equals_IR252=True,function_bytes=len(y),sha256=hashlib.sha256(y).hexdigest(),limitation='New Boolean facts remove branches in the root and fallback numeric functions, but no new branch in this current hot query.'))
(P/'native-text-proofs.json').write_text(json.dumps(proof,indent=2)+'\n')
s=(base/'run-cohorts.py').read_text()
a=s.index('qualified = {}');b=s.index('\ndef quiet():',a)
qualification='''qualified={}
for label,version,passed,selected in [
 ('parent-frame-runtime-counts','v1',887,392),
 ('boolean-loop-integration','v3',888,393)]:
 T=Path('/private/tmp/mech-related-'+label+'-tests-20261107-'+version)
 t=read(T/'test-run.json');assert t['expected_failures_exact']and not t['new_failures']and str(passed)+' passed; 2 failed'in t['summary'][0]
 ci=read(T/'ci-selected-inventory-audit.json');assert ci['expected']==ci['listed']==ci['executed_passed']==selected and ci['all_names_match']
 C=Path('/private/tmp/mech-related-'+label+'-regressions-20261107-'+version)
 for kind,count in [('scalar',128),('one-kib',5)]:
  c=read(C/(kind+'-comparison.json'));assert c['rows']==count and c['all_nontiming_fields_equal_IR240']
 qualified[label]=dict(GPU_passed=passed,inherited_failures=2,new_failures=0,CI_executed=selected,controls_nontiming_exact=133,Rust_edge_outputs_exact=62)
profile=read(Path('/private/tmp/mech-related-parent-frame-counts-post-json-allocation-profile-20261107-v1/summary.json'))['variants']['255']
assert profile['requests']==3758 and profile['requested_bytes']==8012356 and profile['unattributed_requests']==0
assert not any('CompilerFrameData'in v['symbol']for v in profile['all_inclusive_symbols'])
'''
s=s[:a]+qualification+s[b:]
s=s.replace('mech-related-borrowed-parent-frame-metadata-cohort-20261107-v2',P.name)
s=s.replace('borrowed-parent-frame-metadata-randomized','boolean-loops-and-parent-counts-randomized').replace('borrowed-parent-frame-metadata-resources','boolean-loops-and-parent-counts-resources').replace('size-native-borrowed-parent-frame-metadata','size-native-boolean-loops-and-parent-counts')
s=s.replace('mech-related-native-borrowed-parent-frame-metadata-latency','mech-related-native-boolean-loops-and-parent-counts-latency').replace('mech-related-native-borrowed-parent-frame-metadata-resources','mech-related-native-boolean-loops-and-parent-counts-resources')
s=s.replace('9902537','9902577').replace('990253','990257').replace('248','310').replace('63','77').replace('variants=8','variants=10').replace('variants=9','variants=11')
(P/'run-cohorts.py').write_text(s)
print(json.dumps(dict(latency_variants=10,resource_variants=11,native_proofs=proof)))
