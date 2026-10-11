#!/usr/bin/env python3
"""Check new general operations on non-benchmark programs and full CLBG output."""
import importlib.util,json,subprocess
from pathlib import Path
spec=importlib.util.spec_from_file_location('runner',Path(__file__).with_name('run_remaining.py'))
r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
HERE=r.CLBG/'remaining-conformance';HERE.mkdir(exist_ok=True)

def build(name,source):
    path=HERE/(name+'.mec');path.write_text(source)
    return r.prepare(HERE,dict(id=name,language='mech',path=path.name))
def run(row,args=(),input=b'',env=None):
    command=[s for s in row['command'] if '{n}' not in s]+list(map(str,args))
    return subprocess.run(command,input=input,capture_output=True,env=r.ENV|(env or {}),check=True).stdout

def main():
    results=[]
    row=build('generic-regex','''raw := io/read()
count := regex/count-bytes(raw, "[A-Z]{2}[0-9]+")
changed := regex/replace-bytes(raw, "([A-Z]{2})([0-9]+)", "$1")
io/write(string/concat(string/concat(number/format(count, 0), ":"), changed))
''')
    data=b'AA14--bb07--XX99--\xff'*4000
    expected=b'8000:'+b'$1--bb07--$1--\xff'*4000
    for enabled in ('0','1'):
        for reuse in ('0','1'):
            assert run(row,input=data,env={'MECH_SEQUENCE_ASYNC':enabled,'MECH_SEQUENCE_BUFFER_REUSE':reuse})==expected
            results.append(dict(case='arbitrary byte regex, literal replacement and independent counts',async_policy=enabled,buffer_reuse=reuse))
    empty=build('empty-regex','''raw := io/read()
io/write(regex/replace-bytes(raw, "", "-"))
''')
    assert run(empty,input=b'a\xff')==b'-a-\xff-'
    results.append(dict(case='zero-length regex matches on non-UTF8 input'))
    fib=build('fibonacci','''More(i<u64>, n<u64>, a<bigint>, b<bigint>, scratch<bigint>) = (more<bool>) :=
  more := i < n.
Step(i<u64>, n<u64>, a<bigint>, b<bigint>, scratch<bigint>) = (next<u64>, limit<u64>, aa<bigint>, bb<bigint>, ss<bigint>) :=
  sum := bigint/add-into(scratch, a, b)
  aa := bigint/assign(a, b)
  bb := bigint/assign(b, sum)
  ss := scratch
  next := i + 1
  limit := n.
(i, n, a, b, scratch) := control/while((0, io/argument-u64(1), bigint/new(0), bigint/new(1), bigint/new(0)), More, Step)
denominator := bigint/new(1)
scaled := bigint/multiply-u64-into(denominator, denominator, 1000000000000000000)
small := bigint/floor-u64(a, scaled)
printed := io/print("{}\\n", small)
literal := io/print("done\\n")
''')
    a,b=0,1
    for _ in range(150):a,b=b,a+b
    assert a.bit_length()>100
    assert run(fib,args=[150])==f'{a//10**18}\ndone\n'.encode()
    results.append(dict(case='source Fibonacci loop, reused arbitrary-precision cells and literal-only printing',bits=a.bit_length()))
    digits=''.join(line[:10] for line in (r.CLBG/'pidigits/inputs/output-full.txt').read_text().splitlines())
    for source in ('pidigits.mec','pidigits-gated.mec','pidigits-advance.mec'):
        row=r.prepare(r.CLBG/'pidigits',dict(id='conformance-'+source[:-4],language='mech',path=source))
        for n in (0,1,2,9,10,11,27,30,97,1000,10000):
            expected=b''.join((digits[start:min(start+10,n)].ljust(10)+'\t:'+str(min(start+10,n))+'\n').encode() for start in range(0,n,10))
            assert run(row,args=[n])==expected,(source,n)
            results.append(dict(case=source,n=n,complete_output=True))
    for enabled in ('0','1'):
        for reuse in ('0','1'):
            here=r.CLBG/'regexredux'
            row=r.prepare(here,dict(id='conformance-regex-'+enabled+'-'+reuse,language='mech',path='regexredux.mec',runtime_environment={'MECH_SEQUENCE_ASYNC':enabled,'MECH_SEQUENCE_BUFFER_REUSE':reuse}))
            for full in (False,True):
                r.execute(here,row,full)
                results.append(dict(case='regexredux.mec',async_policy=enabled,buffer_reuse=reuse,full=full,complete_output=True))
    report=dict(compiler_sha256=r.sha(r.MECH),cases=results)
    (HERE/'results.json').write_text(json.dumps(report,indent=2)+'\n')
    print('Passed',len(results),'general feature and complete output checks')

if __name__=='__main__':main()
