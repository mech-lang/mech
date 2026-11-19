"""Untimed diagnostic block counters; shipping objects come only from source compilation."""
from pathlib import Path
from runpy import run_path
from hashlib import sha256
from types import ModuleType,SimpleNamespace
import ast,json,struct,sys
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent;Q=LAB.parent
PACKET=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116')
sys.path.insert(0,str(Q));sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
# Reuse only the original parser/relocation helpers. Importing the historical
# outlining script as a whole would open unrelated, now-missing temp artifacts.
obj_file=PACKET.parent/'compact-size/compact_object.py';branch_file=Q/'outline-compiled-aarch64-root.py'
def functions(path,names):
 tree=ast.parse(path.read_text());nodes=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names];assert {n.name for n in nodes}==set(names)
 scope={'unpack_from':struct.unpack_from};exec(compile(ast.Module(body=nodes,type_ignores=[]),str(path),'exec'),scope);return {n:scope[n] for n in names}
layout=ModuleType('outline-compiled-aarch64-root');layout.obj=SimpleNamespace(**functions(obj_file,['tables','relocs']))
for key,value in functions(branch_file,['branch_field','branch_target','branch_opcode']).items():setattr(layout,key,value)
sys.modules[layout.__name__]=layout
h=run_path(str(Q/'current-roadmap/shared-heap-call-setup/evaluate.py'),run_name='natural4k_counter_tools');rt=h['runtime'];study=rt.study;driver=rt.driver
study.HERE=ROOT;study.rebase.HERE=ROOT
training=('prefix-natural-1024','prefix-natural-4096','prefix-natural-8192','prefix-random-4096','prefix-random-8192','prefix-random-1048576','16384-distribution-change')
roles=sys.argv[1:] or ['matcher-vector-default','matcher-word-default','matcher-vector-no-prefix-default','matcher-vector-no-reuse-default']
normalizer_file=LAB/'collector-read-lifetimes/canonical-counter-input.py'
normalizer_source=normalizer_file.read_text().split("link=run_path",1)[0]
normalizer={'__file__':str(normalizer_file),'__name__':'scorer_counter_coordinates'};exec(compile(normalizer_source,str(normalizer_file),'exec'),normalizer)
normal=normalizer['normalize']
symbol='_mech_compact_numeric_2821_vector'
shape_source=LAB/'natural4k-runtime-target/rebase-spill-pair-profiles.py'
shapes={'__file__':str(shape_source)};exec(compile(shape_source.read_text().split('base=read(LAB/')[0],str(shape_source),'exec'),shapes);shapes['OUT']=ROOT
shape=lambda role:shapes['shape'](role,symbol[1:])
def profile_matcher(role,shape,folder):
 from collections import defaultdict
 training=json.loads((folder/'counts.json').read_text());heads=shape['heads'];ids={h:i for i,h in enumerate(heads)};graph=[tuple(dict.fromkeys(dst for dst in row[1:] if dst is not None)) for row in shape['blocks']];weights=defaultdict(float);hotness=[0.]*len(graph);traces={};incoming=defaultdict(list)
 for src,ds in enumerate(graph):
  for dst in ds:incoming[dst].append(src)
 model_trace={'cases':{case:{'blocks':[{'address':hex(ids[int(b['address'],16)-shape['atom']['start']]),'visits':b['visits']} for b in row['blocks']]} for case,row in training['cases'].items()}}
 model_source=Q/'profile-matcher-fallthroughs.py';nodes=[n for n in ast.parse(model_source.read_text()).body if isinstance(n,ast.FunctionDef) and n.name=='edge_flows'];assert len(nodes)==1;model={'defaultdict':defaultdict,'trace':model_trace,'heads':list(range(len(graph))),'successors':dict(enumerate(graph)),'incoming':incoming};exec(compile(ast.Module(body=nodes,type_ignores=[]),str(model_source),'exec'),model);resolved={}
 for case,row in training['cases'].items():
  counts,trace,known=model['edge_flows'](case);peak=max(counts.values());assert peak>0;resolved[case]={'exactly_resolved_edges':known,'total_edges':len(trace)}
  for at,n in counts.items():hotness[at]+=n/peak
  for edge,n in trace.items():weights[edge]+=n/peak
  traces[case]=trace
 ranked={}
 for src,ds in enumerate(graph):
  for dst in ds:
   fall=shape['blocks'][src][2];potential=weights[src,fall] if len(ds)==2 else weights[src,dst];ranked[src,dst]=potential+.35*weights[src,dst]+.001
 order=study.fit.refine(study.scheduler.order_by_weight(ranked,hotness),ranked);profile=ROOT/(role+'-matcher-profile.json');profile.write_text(json.dumps({'order':[heads[i] for i in order],'literal_offset':shape['pool'],'thread_jumps':True},indent=2)+'\n');prediction={n:{'before':study.scheduler.overhead(shape['blocks'],list(range(len(graph))),trace),'after':study.scheduler.overhead(shape['blocks'],order,trace)} for n,trace in traces.items()}
 (ROOT/role/'layout-proof-matcher.json').write_text(json.dumps({'shape':shape,'training_sha256':sha(folder/'counts.json'),'profile_sha256':sha(profile),'estimated_unconditional_branches':prediction,'resolved_edge_counts':resolved,'edge_model_source_sha256':sha(model_source),'edge_model':'Exact constrained edges and explicitly estimated residual edges. Layout heuristic only; not a latency or exact edge-count claim.'},indent=2)+'\n');print(role,'matcher profile prepared',flush=True);return profile

source_shapes={}
for role in roles:
 actual=ROOT/role/'bound'
 if not (actual/'runtime.json').exists():driver.link.link_worker(LAB/'collector-read-lifetimes/placement/immutable-profiled/bound',ROOT/role/'frontend',actual)
 recipe=json.loads((actual/'runtime.json').read_text());recipe.pop('size_bytes',None);recipe.pop('encoder_sha256',None);recipe['encoder_text_bytes']=json.loads((ROOT/role/'receipt.json').read_text())['text_bytes'];(actual/'runtime.json').write_text(json.dumps(recipe,indent=2)+'\n')
 if not (actual/'qualification.json').exists():driver.link.check_worker(actual,'scorer-body44-raw-'+role)
 original=shape(role);source_shapes[role]=original
 front=ROOT/role/'canonical-counter-input/frontend';front.mkdir(parents=True,exist_ok=True)
 normalization=normal(ROOT/role/'frontend/program.o',front/'program.o')
 for name in ['metadata.json','program.mecb']:(front/name).write_bytes((ROOT/role/'frontend'/name).read_bytes())
 (front.parent/'canonicalization.json').write_text(json.dumps(normalization,indent=2)+'\n')
 target=front.parent/'bound'
 if not (target/'runtime.json').exists():driver.link.link_worker(actual,front,target)
 assert sha(target/'mech-worker')==sha(actual/'mech-worker'), 'Counter coordinate input must link to the exact same worker'
 if not (target/'qualification.json').exists():driver.link.check_worker(target,'scorer-body44-canonical-'+role)
 ref=shape(role);folder=ROOT/role/'instruction-counts-histogram'
 original=Q/'count-matcher-instructions.py';program=original.read_text();needle='    root = q / f"{label}-native/final-candidate"';assert program.count(needle)==1;program=program.replace(needle,f'    root = Path({str(target)!r})')
 needle='    directory = q / "matcher-dynamic-blocks" / actor';assert program.count(needle)==1;program=program.replace(needle,f'    directory = Path({str(folder)!r})')
 program=program.replace('allow_data_references=literal_start is not None','allow_data_references=True')
 headers={'__name__':'natural4k_source_body_counters','__file__':str(original)};exec(compile(program,str(original),'exec'),headers)
 if not (folder/'canonical-counts.json').exists():
  headers['count'](role,role,training,symbol,ref['pool'])
  (folder/'counts.json').rename(folder/'canonical-counts.json')
 record={'untimed':True,'shipping_encoder_sha256':sha(actual/'program.o'),'diagnostic_coordinate_object_sha256':sha(target/'program.o'),'counter_driver_sha256':sha(original),'branch_helpers':{'source':str(branch_file),'sha256':sha(branch_file)},'object_helpers':{'source':str(obj_file),'sha256':sha(obj_file)},'training_cases':training,'counter_source':program,'scope':'Instrumentation adds counters only to a diagnostic clone. Never use it for latency or deployment.'}
 (folder/'plan.json').write_text(json.dumps(record,indent=2)+'\n')
 raw=folder/'counts.json'
 counts=json.loads((folder/'canonical-counts.json').read_text());canonical_atom=driver.symbols(front/'program.o')['symbols'][symbol];source_atom=source_shapes[role]['atom'];assert {k:canonical_atom[k] for k in ['extent','sha256','relocations']}=={k:source_atom[k] for k in ['extent','sha256','relocations']}
 delta=source_atom['start']-canonical_atom['start']
 for case in counts['cases'].values():
  for block in case['blocks']:block['address']=hex(int(block['address'],16)+delta)
 counts.update(object_sha256=source_shapes[role]['object_sha256'],canonical_counts_sha256=sha(folder/'canonical-counts.json'),address_translation=delta,canonicalization_sha256=sha(front.parent/'canonicalization.json'))
 raw.write_text(json.dumps(counts,indent=2)+'\n')
 print(role,'histogram counts',raw,flush=True)
