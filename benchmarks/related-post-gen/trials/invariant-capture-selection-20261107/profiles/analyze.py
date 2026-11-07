from pathlib import Path
import json,re,subprocess,collections,hashlib
P=Path(__file__).parent
sources={'selected-ir242':Path('/private/tmp/mech-related-conjoined-window-integration-runners-20261107-v1/reuse-4'),'candidate-ir250':Path('/private/tmp/mech-related-invariant-capture-hoisting-runners-20261107-v2/reuse-4')}
for label,base in sources.items():
 p=P/label;lines=(p/'sample.txt').read_text().splitlines();start=next(i for i,s in enumerate(lines)if s.startswith('Sort by top of stack'))+1;rows=[]
 for s in lines[start:]:
  m=re.match(r'^\s+(.+?)\s+\(in (.+?)\)\s+(\d+)\s*$',s)
  if m:rows.append(dict(symbol=m[1],image=m[2],leaf_snapshots=int(m[3])))
  elif rows and not s.strip():break
 assert rows
 threads={int(m[1])for s in lines if(m:=re.search(r'\bThread_(\d+)',s))}
 native=[x for x in rows if x['symbol'].startswith('mech_canonical_program_entry_v1')];native.sort(key=lambda x:x['leaf_snapshots'],reverse=True)
 run=json.loads((p/'run.json').read_text());assert run['exit']==run['sample_exit']==0 and run['iterations']==1500 and run['final_json_exact_Rust']
 summary=dict(scope=run['scope'],profile_timings_excluded=True,iterations=1500,observed_thread_ids=len(threads),collapsed_leaf_rows_reported_at_least5=len(rows),top_leaf_symbols=rows[:25],native_leaf_symbols=native,query_leaf_snapshots=next(x['leaf_snapshots']for x in native if x['symbol']=='mech_canonical_program_entry_v1_numeric_1282'),limits=['Counts are sample observations, not CPU-time percentages.','The profile includes driver cloning/verification and initial load/restore; it is not the strict continuous post-JSON scope.','Threads that start and exit between sampling points may be missed.','Sampling perturbs execution; all profile timings are excluded from performance/resource claims.'])
 (p/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
 cmd=['/opt/homebrew/opt/llvm/bin/llvm-objdump','--disassemble','--no-show-raw-insn','--disassemble-symbols=_mech_canonical_program_entry_v1_numeric_1282',str(base/'build/object.o')]
 x=subprocess.run(cmd,capture_output=True,text=True);assert x.returncode==0,x.stderr;(p/'query.asm').write_text(x.stdout);instructions=[]
 for line in x.stdout.splitlines():
  m=re.match(r'^\s*[0-9a-fA-F]+:\s+([a-z][a-z0-9.]*)\s*(.*)',line)
  if m:instructions.append((m[1],m[2]))
 assert instructions and '_mech_canonical_program_entry_v1_numeric_1282' in x.stdout
 m=json.loads((base/'build/metadata.json').read_text());query=next(x for x in m['compiled_numeric_functions']if x['header']==1282)
 static=dict(header=1282,code_bytes=query['code_bytes'],stack_bytes=query['stack_bytes'],static_instructions=len(instructions),conditional_select_instructions=sum(op in ['csel','csinc','csinv','csneg','cinc','cset','csetm']for op,args in instructions),compare_instructions=sum(op in ['cmp','cmn','ccmp','ccmn']for op,args in instructions),stack_memory_instructions=sum(op.startswith(('ldr','str','ldp','stp','ldur','stur'))and bool(re.search(r'\[(?:sp|x29)(?:,|\])',args))for op,args in instructions),mnemonics=dict(collections.Counter(op for op,args in instructions)),assembly_sha256=hashlib.sha256(x.stdout.encode()).hexdigest(),dynamic_instruction_claim=False)
 (p/'query-instructions.json').write_text(json.dumps(static,indent=2)+'\n');print(label,'observed threads',len(threads),'query leaf snapshots',summary['query_leaf_snapshots'],'query static',{k:static[k]for k in ['code_bytes','stack_bytes','static_instructions','conditional_select_instructions','stack_memory_instructions']})
