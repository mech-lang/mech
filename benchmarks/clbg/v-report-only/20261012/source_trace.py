"""Unscored V work-trace proposal; never builds, launches or scores a program.

Instrumentation emits a distinct V copy, preserving the original arithmetic,
map updates and stdout. Reconciliation requires the actual held timing mutex.
Root owns the guarded build/execute and seals all source/runtime/input hashes.
"""
from collections import Counter
import argparse
import fcntl
import json
import os
from pathlib import Path
import re

HERE=Path(__file__).resolve().parent
LOCK=Path('/tmp/mech-clbg-timing.lock')
WIDTHS=(1,2,3,4,6,12,18)
QUERIES=('GGT','GGTA','GGTATT','GGTATTTTAATT','GGTATTTTAATTTATAGT')
VARIANTS={'knucleotide':{'compact','optimized'},'pidigits':{'compact','optimized','published-package'}}
TRACE_ENV='MECH_V_WORK_TRACE'

def _owned_mutex(fd):
    assert fd is not None,'Actual trace reconciliation requires the held common timing mutex'
    expected=os.stat(LOCK);actual=os.fstat(fd)
    assert (expected.st_dev,expected.st_ino)==(actual.st_dev,actual.st_ino),'Wrong timing mutex descriptor'
    # A separate open description must already be blocked. Otherwise a supplied
    # but unlocked descriptor would accidentally acquire the mutex here.
    with LOCK.open('a+') as probe:
        try:
            fcntl.flock(probe.fileno(),fcntl.LOCK_EX|fcntl.LOCK_NB)
        except BlockingIOError:
            pass
        else:
            fcntl.flock(probe.fileno(),fcntl.LOCK_UN)
            raise AssertionError('The supplied timing mutex must already be held')
    fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB)

def _insert_before(source,statement,addition):
    expression=re.compile(r'^([ \t]*)'+re.escape(statement)+r'[ \t]*$',re.MULTILINE)
    matches=list(expression.finditer(source));assert len(matches)==1,('Source anchor is not unique',statement,len(matches))
    match=matches[0];indent=match.group(1)
    inserted='\n'.join(indent+line if line else '' for line in addition.splitlines())+'\n'
    return source[:match.start()]+inserted+source[match.start():]

def _insert_after(source,statement,addition,*,inside_block=False):
    expression=re.compile(r'^([ \t]*)'+re.escape(statement)+r'[ \t]*$',re.MULTILINE)
    matches=list(expression.finditer(source));assert len(matches)==1,('Source anchor is not unique',statement,len(matches))
    match=matches[0];indent=match.group(1)+('    ' if inside_block else '')
    inserted='\n'+'\n'.join(indent+line if line else '' for line in addition.splitlines())
    return source[:match.end()]+inserted+source[match.end():]

MAP_COMPACT=r'''
// Unscored audit copy only; this helper is absent from the measured source.
fn audit_trace_maps(tables []map[string]u64, sequence_length int) {
    path := os.getenv('MECH_V_WORK_TRACE')
    if path.len == 0 {
        panic('An isolated work trace path is required')
    }
    mut audit := os.create(path) or { panic(err) }
    defer {
        audit.close()
    }
    audit.writeln('TRACE_V1|knucleotide|compact|${tables.len}|${sequence_length}') or { panic(err) }
    for i, table in tables {
        mut total := u64(0)
        for key, value in table {
            audit.writeln('MAP|${lengths[i]}|${key}|${value}') or { panic(err) }
            total += value
        }
        audit.writeln('TABLE|${lengths[i]}|${table.len}|${total}') or { panic(err) }
    }
    audit.writeln('END_V1') or { panic(err) }
}
'''
MAP_OPTIMIZED=r'''
// Unscored audit copy only; maps are dumped by the parent after every join.
fn audit_trace_maps(tables []CountResult, sequence_length int) {
    path := os.getenv('MECH_V_WORK_TRACE')
    if path.len == 0 {
        panic('An isolated work trace path is required')
    }
    mut audit := os.create(path) or { panic(err) }
    defer {
        audit.close()
    }
    audit.writeln('TRACE_V1|knucleotide|optimized|${tables.len}|${sequence_length}') or { panic(err) }
    for table in tables {
        mut total := u64(0)
        for key, value in table.counts {
            audit.writeln('MAP|${table.k}|${key}|${value}') or { panic(err) }
            total += value
        }
        audit.writeln('TABLE|${table.k}|${table.counts.len}|${total}') or { panic(err) }
    }
    audit.writeln('END_V1') or { panic(err) }
}
'''

PI_OPEN=r'''
path := os.getenv('MECH_V_WORK_TRACE')
if path.len == 0 {
    panic('An isolated work trace path is required')
}
mut audit_trace := os.create(path) or { panic(err) }
audit_trace.writeln('TRACE_V1|pidigits|WORK_VARIANT|${n}') or { panic(err) }
defer {
    audit_trace.writeln('END_V1') or { panic(err) }
    audit_trace.close()
}
mut audit_candidate := 0
'''.strip()
PI_COMPACT_DIV=r'''
audit_candidate++
audit_trace.writeln('DIV|${audit_candidate}|lower|${numerator3.str()}|${t.str()}|${digit3.str()}') or { panic(err) }
audit_trace.writeln('DIV|${audit_candidate}|upper|${numerator4.str()}|${t.str()}|${digit4.str()}') or { panic(err) }
'''.strip()
PI_BASIS_DIV=r'''
audit_candidate++
audit_trace.writeln('DIV|${audit_candidate}|upper|${n1.str()}|${d.str()}|${u.str()}') or { panic(err) }
audit_trace.writeln('DIV|${audit_candidate}|lower|${n2.str()}|${d.str()}|${v.str()}') or { panic(err) }
'''.strip()

def instrument_text(source,benchmark,variant):
    """Source edit only: do not infer a new anchor after a program revision."""
    assert benchmark in VARIANTS and variant in VARIANTS[benchmark]
    assert TRACE_ENV not in source and 'audit_trace_' not in source,'Never reinstrument an audit copy'
    original=source
    if benchmark=='knucleotide':
        source=_insert_before(source,'for index in 0 .. 2 {','audit_trace_maps(tables, sequence.len)')
        source=source.rstrip()+'\n'+(MAP_COMPACT if variant=='compact' else MAP_OPTIMIZED)
        additions=['Parent dumps every completed map before frequency/query output; optimized workers have all joined.','Typed map iterators expose every actual key/count and exact map.len, with seven table totals.']
    else:
        # Open before the zero-digit guard; defer also records an empty run.
        source=_insert_before(source,'if n <= 0 {',PI_OPEN.replace('WORK_VARIANT',variant))
        if variant=='compact':
            source=_insert_after(source,'digit4 := numerator4 / t',PI_COMPACT_DIV)
            source=_insert_after(source,'if digit3 == digit4 {',"audit_trace.writeln('ACCEPT|${audit_candidate}|${digit3.str()}|${printed + 1}') or { panic(err) }",inside_block=True)
        else:
            source=_insert_after(source,'v = n2 / d',PI_BASIS_DIV)
            source=_insert_after(source,'if u_int == v_int {',"audit_trace.writeln('ACCEPT|${audit_candidate}|${u.str()}|${digits_printed + 1}') or { panic(err) }",inside_block=True)
        additions=['Both quotient values are logged immediately after their actual source divisions.','Acceptance is logged only inside the original digit-emission branch.','Original numerator construction, arithmetic ordering, predicate, output and loop remain unchanged.']
    assert source!=original
    return source,dict(status='authored-unbuilt-unqualified-trace-copy',benchmark=benchmark,variant=variant,
        scheme='TRACE_V1',trace_environment=TRACE_ENV,changes=additions,
        full_workload=False,preparation_only=True,never_chart_eligible=True,
        full_performance_run=False,stdout_policy='Must still match the complete independent bounded oracle.',
        original_source_mutated=False,trace_file_policy='Fresh isolated file; dump complete values, never use the truncated stderr capture.')

def _lines(trace,benchmark,variant):
    lines=trace.decode('ascii').splitlines()
    assert lines and lines[-1]=='END_V1' and lines.count('END_V1')==1,'Incomplete or duplicated trace boundary'
    header=lines[0].split('|')
    assert header[:3]==['TRACE_V1',benchmark,variant]
    return header,lines[1:-1]

def _sequence(data):
    assert len(data)<=1024*1024,'Work traces are bounded qualification, not full workloads'
    selected=False;parts=[]
    # V's reader recognizes CR/LF line separators and only a space/tab/end
    # THREE token boundary; Python's broader Unicode whitespace is not input.
    for line in re.split(r'\r\n|\r|\n',data.decode('ascii')):
        if line.startswith(';'):continue
        if line.startswith('>'):
            if selected:break
            selected=line=='>THREE' or line.startswith('>THREE ') or line.startswith('>THREE\t')
        elif selected:parts.append(line.upper())
    sequence=''.join(parts)
    assert set(sequence)<=set('ACGT'),'The required nucleotide alphabet is A/C/G/T'
    return sequence

def _decode(key,k):
    value=int(key)
    assert str(value)==key and 0<=value<(1<<(2*k)),'Malformed packed key or unused high bits'
    return ''.join('ACGT'[(value>>(2*(k-index-1)))&3] for index in range(k))

def _nucleotide_output(maps):
    output=[]
    for counts in maps[:2]:
        total=sum(counts.values())
        for word,count in sorted(counts.items(),key=lambda pair:(-pair[1],pair[0])):
            output.append(f'{word} {100.0*count/total:.3f}\n')
        output.append('\n')
    output.extend(f'{counts[word]}\t{word}\n' for counts,word in zip(maps[2:],QUERIES))
    return ''.join(output).encode()

def reconcile_maps(trace,stdin,stdout,variant,*,common_timing_mutex_fd):
    _owned_mutex(common_timing_mutex_fd)
    assert variant in VARIANTS['knucleotide']
    header,events=_lines(trace,'knucleotide',variant)
    sequence=_sequence(stdin)
    assert len(header)==5 and int(header[3])==7 and int(header[4])==len(sequence)
    expected={k:Counter(sequence[i:i+k] for i in range(max(0,len(sequence)-k+1))) for k in WIDTHS}
    observed={k:{} for k in WIDTHS};tables=[];position=0
    for event in events:
        fields=event.split('|');assert position<len(WIDTHS),'Extra map trace table'
        k=WIDTHS[position]
        assert len(fields)==4 and int(fields[1])==k,'Wrong table order or event shape'
        if fields[0]=='MAP':
            word=fields[2] if variant=='compact' else _decode(fields[2],k)
            assert len(word)==k and set(word)<=set('ACGT')
            count=int(fields[3]);assert count>0 and str(count)==fields[3]
            assert word not in observed[k],'Duplicate entry is not an actual complete map dump'
            observed[k][word]=count
        else:
            assert fields[0]=='TABLE'
            assert int(fields[2])==len(observed[k]) and int(fields[3])==sum(observed[k].values())
            assert observed[k]==expected[k],('Actual map differs from all-window oracle',k)
            assert sum(observed[k].values())==max(0,len(sequence)-k+1)
            tables.append(dict(k=k,distinct_keys=len(observed[k]),windows=sum(observed[k].values())))
            position+=1
    assert position==7
    assert stdout==_nucleotide_output([expected[k] for k in WIDTHS]),'Trace copy changed complete benchmark stdout'
    return dict(status='passed',benchmark='knucleotide',variant=variant,sequence_length=len(sequence),
        complete_maps=tables,all_seven_maps_reconciled=True,all_overlapping_windows=True,
        first_two_maps_included=True,stdout_unchanged=True,preparation_only=True,
        never_chart_eligible=True,full_performance_run=False)

def _pi_output(digits):
    return ''.join(f'{"".join(str(d) for d in digits[start:start+10]):<10}\t:{min(start+10,len(digits))}\n'
        for start in range(0,len(digits),10)).encode()

def reconcile_pi(trace,stdout,n,variant,*,common_timing_mutex_fd):
    _owned_mutex(common_timing_mutex_fd)
    assert variant in VARIANTS['pidigits'] and type(n) is int and 0<=n<=256,'Only bounded independent pi traces are admitted'
    header,events=_lines(trace,'pidigits',variant)
    assert len(header)==4 and int(header[3])==n
    q,r,t,k=1,0,1,1
    digits=[];candidate=0;cursor=0
    order=('lower','upper') if variant=='compact' else ('upper','lower')
    while cursor<len(events):
        assert len(digits)<n,'Extra division after requested digit count'
        candidate+=1
        numerators={'lower':3*q+r,'upper':4*q+r}
        quotients={}
        for role in order:
            assert cursor<len(events),'Missing actual second division'
            fields=events[cursor].split('|');cursor+=1
            assert len(fields)==6 and fields[:3]==['DIV',str(candidate),role]
            numerator,denominator,quotient=map(int,fields[3:])
            assert numerator==numerators[role]>=0 and denominator==t>0
            assert quotient==numerator//denominator,'Actual quotient differs from independent exact division'
            quotients[role]=quotient
        assert numerators['upper']-numerators['lower']==q>0
        if quotients['lower']==quotients['upper']:
            digit=quotients['lower'];assert 0<=digit<=9
            assert cursor<len(events),'Accepted digit was not traced in the actual output branch'
            fields=events[cursor].split('|');cursor+=1
            assert fields==['ACCEPT',str(candidate),str(digit),str(len(digits)+1)]
            digits.append(digit)
            q,r=10*q,10*(r-digit*t)
        else:
            factor=2*k+1
            q,r,t=q*k,(2*q+r)*factor,t*factor
            k+=1
    assert len(digits)==n and stdout==_pi_output(digits),'Incomplete digits, tail/count or changed stdout'
    return dict(status='passed',benchmark='pidigits',variant=variant,n=n,
        tested_candidates=candidate,actual_divisions=2*candidate,accepted_digits=len(digits),
        both_actual_extracts_per_candidate=True,distinct_endpoint_numerators=True,
        all_numerators_and_quotients_reconciled=True,nonnegative_numerators=True,
        stdout_unchanged=True,preparation_only=True,never_chart_eligible=True,
        full_performance_run=False)

def main():
    parser=argparse.ArgumentParser();sub=parser.add_subparsers(dest='operation',required=True)
    draft=sub.add_parser('draft');draft.add_argument('--benchmark',choices=VARIANTS,required=True)
    draft.add_argument('--variant',required=True);draft.add_argument('--source',type=Path,required=True)
    draft.add_argument('--output',type=Path,required=True)
    review=sub.add_parser('reconcile');review.add_argument('--benchmark',choices=VARIANTS,required=True)
    review.add_argument('--variant',required=True);review.add_argument('--trace',type=Path,required=True)
    review.add_argument('--stdout',type=Path,required=True);review.add_argument('--stdin',type=Path)
    review.add_argument('--n',type=int);review.add_argument('--receipt',type=Path,required=True)
    args=parser.parse_args()
    if args.operation=='draft':
        assert args.source.resolve()!=args.output.resolve(),'Original benchmark source must remain untouched'
        plan_path=args.output.with_suffix('.plan.json')
        assert not args.output.exists() and not plan_path.exists(),'Both trace-copy artifact paths must be fresh'
        text,plan=instrument_text(args.source.read_text(),args.benchmark,args.variant)
        plan.update(original_source=str(args.source.resolve()),trace_source=str(args.output.resolve()),source_hashes='Root must seal both under common mutex before actual preparation.')
        args.output.parent.mkdir(parents=True,exist_ok=True)
        with args.output.open('x') as stream:stream.write(text)
        with plan_path.open('x') as stream:json.dump(plan,stream,indent=2);stream.write('\n')
        print(json.dumps(plan,indent=2));return
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        assert not args.receipt.exists(),'Never overwrite an earlier work-trace receipt'
        trace=args.trace.read_bytes();stdout=args.stdout.read_bytes()
        if args.benchmark=='knucleotide':
            assert args.stdin is not None and args.n is None
            result=reconcile_maps(trace,args.stdin.read_bytes(),stdout,args.variant,common_timing_mutex_fd=lock.fileno())
        else:
            assert args.n is not None and args.stdin is None
            result=reconcile_pi(trace,stdout,args.n,args.variant,common_timing_mutex_fd=lock.fileno())
        result.update(trace=str(args.trace.resolve()),stdout=str(args.stdout.resolve()),
            hash_receipt='Root must independently pin the exact source/runtime/input/trace/stdout before admission of this unscored work review.')
        with args.receipt.open('x') as stream:json.dump(result,stream,indent=2);stream.write('\n')
        print(json.dumps(result,indent=2))

if __name__=='__main__':main()
