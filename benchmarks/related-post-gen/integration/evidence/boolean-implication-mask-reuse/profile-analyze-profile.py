from pathlib import Path
import collections, hashlib, importlib.util, json, re, subprocess

P=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
read=lambda p:json.loads(Path(p).read_text())
def write(p,d):p.write_text(json.dumps(d,indent=2)+'\n')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
spec=importlib.util.spec_from_file_location('audit',R/'audit-native-link.py')
audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)
def numeric_text(bundle):
    blob,sections,symbols=audit.macho(bundle/'object.o')
    section=next(s for s in sections if s['name']=='__text')
    rows=read(bundle/'metadata.json')['compiled_numeric_functions']
    symbols={s['name']:s for s in symbols}
    result={}
    for row in rows:
        symbol=symbols['_'+row['symbol']]
        offset=symbol['value']-section['address']+section['offset']
        result[row['header']]=blob[offset:offset+row['code_bytes']]
    return result
def category(op,args):
    if op.startswith(('ld','st')):
        return 'explicit_stack_memory' if re.search(r'\[(?:sp|x29)(?:,|\])',args) else 'other_memory'
    if op in ['mov','movk','movz','movn','fmov']:
        return 'constant_materialization' if '#' in args else 'register_move'
    if op in ['b','bl','br','blr','braa','brab','blraa','blrab','ret','retab'] or op.startswith(('b.','cb','tb')):
        return 'branch_or_return'
    if op in ['cmp','cmn','tst','fcmp'] or op.startswith('ccmp'):
        return 'comparison'
    return 'other_arithmetic_or_control'
summary=read(P/'summary.json');variants={}
for label,row in summary['rows'].items():
    assert row['all1000_index_arrays_and_final_rust_output_exact'] and row['profiled_timings_excluded']
    symbols=[s for s in row['native_pc_offsets']if re.search(r'_numeric_\d+$',s)]
    command=['/opt/homebrew/opt/llvm/bin/llvm-objdump','--disassemble','--no-show-raw-insn','--disassemble-symbols='+','.join(symbols),str(P/label/'runner-pc')]
    assembly=subprocess.check_output(command,text=True);(P/(label+'-kernels.asm')).write_text(assembly)
    functions={};current=None;base=0
    for line in assembly.splitlines():
        m=re.match(r'([0-9a-f]+) <([^>]+)>:',line)
        if m:current=m[2];base=int(m[1],16);functions[current]={};continue
        m=re.match(r'([0-9a-f]+):\s+([a-z0-9.]+)\s*(.*)',line)
        if m and current:functions[current][int(m[1],16)-base]=(m[2],m[3],line)
    rows={}
    for symbol,instructions in functions.items():
        sampled={int(k):v for k,v in row['native_pc_offsets'][symbol].items()}
        assert sampled.keys()<=instructions.keys()
        counts=collections.Counter()
        for offset,count in sampled.items():counts[category(*instructions[offset][:2])]+=count
        rows[symbol]=dict(static_machine_instruction_count=len(instructions),static_categories=dict(collections.Counter(category(op,args)for op,args,line in instructions.values())),sampled_pc_observations=sum(sampled.values()),sampled_categories=dict(counts),top_offsets=[dict(offset=o,observations=n,instruction=instructions[o][2],window=[instructions[k][2]for k in range(max(0,o-12),o+16,4)if k in instructions])for o,n in sorted(sampled.items(),key=lambda p:-p[1])[:15]])
    names=[s[1:]if s.startswith('__R')else s for s,n in row['top_symbol_observations']]
    demangled=subprocess.check_output(['/opt/homebrew/opt/llvm/bin/llvm-cxxfilt'],input='\n'.join(names)+'\n',text=True).splitlines()
    assert len(demangled)==len(names)
    row['demangled_top_symbol_observations']=[dict(symbol=s,demangled=d,observations=n)for (s,n),d in zip(row['top_symbol_observations'],demangled)]
    row['executable_sha256']=sha(P/label/'runner-pc')
    variants[label]=dict(binary_sha256=row['executable_sha256'],all1000_invocations_exact=True,rows=rows)
write(P/'enriched-summary.json',summary)
write(P/'worker-instruction-observations.json',dict(schema='mech.related-post-gen.native-worker-instruction-observations.v2',compilers={label:row['compiler']for label,row in summary['rows'].items()},profiled_timings_excluded=True,variants=variants,findings=['Every sampled offset uses that profile executable and its actual linked native Object.','Explicit stack loads/stores, rank arithmetic, comparisons and register moves remain observed.','The before/after native function inventory differs; no earlier mapping or source-versus-bookkeeping split is assumed.'],limitations=['Current-PC samples are statistical positions, not dynamic instruction counts or CPU-time shares.','Static counts include cold error paths.','A source location alone does not identify a spill or move as useful arithmetic or bookkeeping.']))
print(json.dumps(dict(profile_invocations_exact=2000,variants=list(variants))))
