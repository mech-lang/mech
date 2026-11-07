from pathlib import Path
import collections, hashlib, json, re, subprocess
P = Path(__file__).parent
def write(p,d): p.write_text(json.dumps(d,indent=2)+'\n')
def category(op,args):
    if op.startswith(('ld','st')):
        return 'explicit_stack_memory' if re.search(r'\[(?:sp|x29)(?:,|\])',args) else 'other_memory'
    if op in ['mov','movk','movz','movn','fmov']:
        return 'constant_materialization' if '#' in args else 'register_move'
    if op in ['b','bl','br','blr','braa','brab','blraa','blrab','ret','retab'] or op.startswith(('b.','cb','tb')): return 'branch_or_return'
    if op in ['cmp','cmn','tst','fcmp'] or op.startswith('ccmp'): return 'comparison'
    return 'arithmetic_or_other'
profile=json.loads((P/'execution-profile.json').read_text());result={}
for label in ['cranelift','llvm']:
    rows=[r for r in profile['rows']if r['variant']==label]
    pcs=collections.defaultdict(collections.Counter)
    for row in rows:
        for name,counts in row['native_pc_offsets'].items(): pcs[name].update({int(k):v for k,v in counts.items()})
    symbols=sorted(pcs)
    command=['/opt/homebrew/opt/llvm/bin/llvm-objdump','--disassemble','--no-show-raw-insn','--disassemble-symbols='+','.join(symbols),str(P/label/'runner-pc')]
    assembly=subprocess.check_output(command,text=True);(P/(label+'-native.asm')).write_text(assembly)
    functions={};current=None;base=0
    for line in assembly.splitlines():
        m=re.match(r'([0-9a-f]+) <([^>]+)>:',line)
        if m: current=m[2];base=int(m[1],16);functions[current]={};continue
        m=re.match(r'([0-9a-f]+):\s+([a-z0-9.]+)\s*(.*)',line)
        if m and current:functions[current][int(m[1],16)-base]=(m[2],m[3],line)
    details={}
    for symbol,instructions in functions.items():
        sampled=pcs[symbol];assert sampled.keys()<=instructions.keys(),(label,symbol,set(sampled)-set(instructions))
        observed=collections.Counter()
        for offset,count in sampled.items():observed[category(*instructions[offset][:2])]+=count
        details[symbol]=dict(static_machine_instruction_count=len(instructions),static_categories=dict(collections.Counter(category(op,args)for op,args,line in instructions.values())),
            register_observations=sum(sampled.values()),observed_instruction_categories=dict(observed),
            top_offsets=[dict(offset=o,observations=n,instruction=instructions[o][2],window=[instructions[k][2]for k in range(max(0,o-20),o+24,4)if k in instructions])for o,n in sampled.most_common(25)])
    result[label]=dict(executable_sha256=hashlib.sha256((P/label/'runner-pc').read_bytes()).hexdigest(),functions=details)
write(P/'native-instruction-observations.json',dict(schema='mech.related-post-gen.backend-instruction-observations.v1',variants=result,all4000_invocations_exact=True,
    limitations=['PC observations are not executed-instruction counts or CPU-time shares.','Static counts include cold error paths and placement padding.','Two new diagnostic executable links, preserving audited native Objects; no diagnostic timings used as benchmark results.']))
for label,row in result.items():
    f=row['functions']['_mech_canonical_program_entry_v1_numeric_1282']
    print(label, json.dumps({k:v for k,v in f.items()if k!='top_offsets'},indent=2))
    print(json.dumps(f['top_offsets'][:8],indent=2))
