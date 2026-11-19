"""Source-emitted fresh matcher layout after unsigned-guard proof; unchanged profiles need graph proof."""
from pathlib import Path
from collections import defaultdict
from runpy import run_path
from hashlib import sha256
import ast,json,os,subprocess,sys
HERE=Path(__file__).resolve().parent;F=HERE/'immutable-only';S=HERE.parent/'natural4k-runtime-target/hot-matcher-facts';Q=HERE.parent.parent;ORIGINAL=HERE.parent/'natural4k-runtime-target';read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
source=ORIGINAL/'rebase-spill-pair-profiles.py';namespace={'__file__':str(source)};exec(compile(source.read_text().split('base=read(LAB/')[0],str(source),'exec'),namespace)
rt=run_path(str(Q/'current-roadmap/shared-heap-call-setup/evaluate.py'),run_name='guard_layout_helpers')['runtime'];study=rt.study
base=read(S/'guards-profiled/receipt.json');trial=read(F/'immutable-defaults/receipt.json');entries=[];proof=[]
for item in base['flags']['MECH_COMPACT_PHASE_BLOCK_LAYOUT'].split(';'):
 symbol,path=item.split('@',1)
 if symbol.endswith('_475_vector'):continue
 namespace['OUT']=S;a=namespace['shape']('guards-profiled',symbol);namespace['OUT']=F;b=namespace['shape']('immutable-defaults',symbol);assert a['blocks']==b['blocks'] and a['heads']==b['heads'] and a['pool']==b['pool'];entries.append(item);proof.append({'symbol':symbol,'same_ordered_graph':True,'control':a,'candidate':b,'profile_sha256':sha(path)})
namespace['OUT']=F;shape=namespace['shape']('immutable-defaults','mech_compact_numeric_475_vector');training_file=F/'immutable-defaults/matcher-counts/counts.json';training=read(training_file);canonical=F/'immutable-defaults/canonical-counter-input';normalization=read(canonical/'canonicalization.json');assert normalization['source_sha256']==trial['object_sha256'] and normalization['diagnostic_sha256']==training['object_sha256'];original_atom=namespace['emit'].symbols(F/'immutable-defaults/frontend/program.o')['symbols']['_mech_compact_numeric_475_vector'];canonical_atom=namespace['emit'].symbols(canonical/'frontend/program.o')['symbols']['_mech_compact_numeric_475_vector'];assert original_atom['sha256']==canonical_atom['sha256'] and original_atom['extent']==canonical_atom['extent'] and original_atom['relocations']==canonical_atom['relocations'];assert sha(canonical/'bound/mech-worker')==sha(F/'immutable-defaults/bound/mech-worker');heads=shape['heads'];ids={h:i for i,h in enumerate(heads)};graph=[tuple(dict.fromkeys(dst for dst in row[1:] if dst is not None)) for row in shape['blocks']];weights=defaultdict(float);hotness=[0.]*len(graph);traces={}
# The historical matcher model explicitly estimates edges that block visits
# cannot uniquely determine. This is a layout heuristic, not exact edge timing.
model_source=Q/'profile-matcher-fallthroughs.py';tree=ast.parse(model_source.read_text());fn=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='edge_flows'];assert len(fn)==1
incoming=defaultdict(list)
for src,ds in enumerate(graph):
 for dst in ds:incoming[dst].append(src)
model_trace={'cases':{case:{'blocks':[{'address':hex(ids[int(b['address'],16)-canonical_atom['start']]),'visits':b['visits']} for b in row['blocks']]} for case,row in training['cases'].items()}}
model={'defaultdict':defaultdict,'trace':model_trace,'heads':list(range(len(graph))),'successors':dict(enumerate(graph)),'incoming':incoming};exec(compile(ast.Module(body=fn,type_ignores=[]),str(model_source),'exec'),model);resolved={}
for case,row in training['cases'].items():
 counts,trace,known=model['edge_flows'](case);peak=max(counts.values());assert peak>0;resolved[case]={'exactly_resolved_edges':known,'total_edges':len(trace)}
 for at,n in counts.items():hotness[at]+=n/peak
 for edge,n in trace.items():weights[edge]+=n/peak
 traces[case]=trace
ranked={}
for src,ds in enumerate(graph):
 for dst in ds:
  fall=shape['blocks'][src][2];potential=weights[src,fall] if len(ds)==2 else weights[src,dst];ranked[src,dst]=potential+.35*weights[src,dst]+.001
order=study.fit.refine(study.scheduler.order_by_weight(ranked,hotness),ranked);profile=F/'matcher-layout.json';profile.write_text(json.dumps({'order':[heads[i] for i in order],'literal_offset':shape['pool'],'thread_jumps':True},indent=2)+'\n');entries.append('mech_compact_numeric_475_vector@'+str(profile));predictions={n:{'before':study.scheduler.overhead(shape['blocks'],list(range(len(graph))),trace),'after':study.scheduler.overhead(shape['blocks'],order,trace)} for n,trace in traces.items()}
(F/'layout-proof.json').write_text(json.dumps({'shape':shape,'canonical_counter_object':normalization,'canonical_atom':canonical_atom,'original_atom':original_atom,'original_and_canonical_worker_sha256':sha(canonical/'bound/mech-worker'),'training_sha256':sha(training_file),'profile_sha256':sha(profile),'unchanged_other_profiles':proof,'estimated_unconditional_branches':predictions,'resolved_edge_counts':resolved,'edge_model_source_sha256':sha(model_source),'edge_model':'Historical matcher heuristic: exact constrained edges plus min(residual outgoing, residual incoming)/2 for unresolved edges. Estimates only choose layout; they are not exact counts or latency.','scope':'Untimed source-profile preparation. Estimated overhead is not measured latency.'},indent=2)+'\n')
folder=F/'immutable-profiled';folder.mkdir();front=folder/'frontend';flags=trial['flags']|{'MECH_COMPACT_PHASE_BLOCK_LAYOUT':';'.join(entries)};command=[*trial['compile']];command[2]=str(front)
with (folder/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
layout=namespace['emit'].symbols(front/'program.o');artifacts={n:sha(front/n) for n in ['program.o','metadata.json','program.mecb']};assert artifacts['program.mecb']==trial['bytecode_sha256'];record=trial|{'source_commit':'fb6bbe11695f05f7b071280e6f8dcc5648b198bf','source_pin':'fb6bbe11695f05f7b071280e6f8dcc5648b198bf','command':command,'compile':command,'flags':flags,'artifacts':artifacts,'object_sha256':artifacts['program.o'],'metadata_sha256':artifacts['metadata.json'],'bytecode_sha256':artifacts['program.mecb'],'text_bytes':layout['text_bytes'],'layout':layout,'qualification':'Fresh profile emitted; linked outputs/errors/recovery/stack/runtime/complete build still required.'};(folder/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');print('Immutable-only-profiled encoder',layout['text_bytes'],artifacts['program.o'],flush=True);print('Untimed profile prediction',predictions,flush=True)

checked=F/'immutable-profiled-checked';checked.mkdir();checked_front=checked/'frontend';checked_command=command.copy();checked_command[2]=str(checked_front)
with (checked/'compile.log').open('w') as log:subprocess.run(checked_command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags|{'MECH_COMPACT_PAIR_REGISTER_SPILLS':'checked'},stdout=log,stderr=log,check=True)
checked_artifacts={n:sha(checked_front/n) for n in artifacts};assert checked_artifacts==artifacts;(checked/'checker-match.json').write_text(json.dumps({'artifacts':checked_artifacts,'compiler_sha256':sha(command[0]),'command':checked_command,'flags':flags|{'MECH_COMPACT_PAIR_REGISTER_SPILLS':'checked'}},indent=2)+'\n');print('Allocation checker exactly reproduces all profiled artifacts.',flush=True)
