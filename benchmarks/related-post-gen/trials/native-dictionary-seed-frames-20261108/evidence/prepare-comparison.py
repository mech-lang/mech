from pathlib import Path
import copy,hashlib,json
P=Path(__file__).parent;B=Path('/private/tmp/mech-related-generic-native-dictionary-20261108')
def read(p):return json.loads(p.read_text())
def write(p,d):p.write_text(json.dumps(d,indent=2)+'\n')
rule=read(B/'prospective-rule.json');rule.update(scope='IR305 closed nonempty String/U64 Map seed uses a genuine admitted private native frame through retained-owner initialization; unchanged source and Cranelift; IR304 immediate and selected processing parent; IR295 retained control',candidate='mech-ir305-native-dictionary-seed-frames-4',immediate_parent='mech-ir304-generic-native-dictionary-4',selected_control='mech-ir304-generic-native-dictionary-4',emission_control='mech-ir305-canonical-seed-control-4',seed=9904100,resource_seed=99041001,confirmation_seed=9904101,confirmation_resource_seed=99041011,bootstrap_seed=990410011,confirmation_bootstrap_seed=990410111,processing_measured_yet=False,latency_variants=31,resource_variants=32)
assert not (P/'prospective-rule.json').exists();write(P/'prospective-rule.json',rule)
c=read(B/'latency-config.json');c['authoritative_latency_comparison']='comparison-native-dictionary-seed-frames-20261108-5000-m3.json';c['prospective_performance_rule']=rule
for name,root,category in [(rule['candidate'],'/private/tmp/mech-related-native-dictionary-seed-frames-runners-20261108-v1/histogram-reuse','IR305_genuine_admitted_native_dictionary_seed_frame'),(rule['emission_control'],'/private/tmp/mech-related-native-dictionary-seed-frames-emission-control-20261108/histogram-reuse','IR305_same_Engine_runtime_only_native_seed_frame_producer_emission_disabled')]:
 v=copy.deepcopy(c['implementations'][-2]);v.update(name=name,category=category,executable=root+'/runner-aot',source=root+'/build/source.mec',receipt=root+'/runner-aot.build.json');v.pop('executable_sha256',None);v.pop('executable_file_bytes',None)
 f=Path(v['executable']);assert f.exists();v.update(executable_sha256=hashlib.sha256(f.read_bytes()).hexdigest(),executable_file_bytes=f.stat().st_size);c['implementations'].append(v)
assert len(c['implementations'])==31 and len({x['name']for x in c['implementations']})==31
assert sum(v['kind']=='upstream'for v in c['implementations'])==22
write(P/'latency-config.json',c)
for n in ['run-cohort.py','analyze.py']:
 s=(B/n).read_text().replace('generic-native-dictionary','native-dictionary-seed-frames').replace('opt-20261108-v6','opt-20261108-v4')
 if n=='run-cohort.py':
  s=s.replace("['actual_CI_passed']==460","['actual_CI_passed']==466").replace("assert read(P/'Engine-component-qualification.json')['only_visibility_changed']","assert read(P/'Engine-component-qualification.json')['all138_native_collection_contracts_passed']").replace("assert read(P/'control-parent-parity.json')['all_nontiming_fields_exact']","assert read(P/'control-parent-parity.json')['all_nontiming_fields_exact']")
 else:s=s.replace("len(lat['samples'])==29*31 and len(res['samples'])==30*7","len(lat['samples'])==31*31 and len(res['samples'])==32*7")
 (P/n).write_text(s)
print('prospective rule and31variant latency config written; no performance sampling yet')
