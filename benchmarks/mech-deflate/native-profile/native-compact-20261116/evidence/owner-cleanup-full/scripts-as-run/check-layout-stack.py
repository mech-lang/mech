from pathlib import Path
import json
import os
import subprocess
import sys

q=Path(__file__).resolve().parent
base=Path('/private/tmp/mech-overwrite-reuse-20261114/final')
link=json.loads((q/'pinned-host-link'/'link.json').read_text())['command']
original=next(Path(item) for item in link if item.endswith('/mech-worker.rs'))
c_object=q/'controller-stack-probe.o'
subprocess.run(['cc','-std=gnu11','-O2','-Wall','-Wextra','-Werror','-c',
                q/'controller-stack-probe.c','-o',c_object],check=True)
src=original.read_text()
src=src.replace('mech_compact_closed_v2(', 'mech_probe_native_stack(')
src=src.replace('        output: *mut u64,','        output: *mut u64,\n        sample: u64,\n        depth: *mut usize,')
src=src.replace('        let status = unsafe {','        let mut depth = 0;\n        let status = unsafe {')
src=src.replace('                output.as_mut_ptr(),','                output.as_mut_ptr(),\n                sample as u64,\n                &mut depth,')
src=src.replace('        let returned = if status.status == 0 {',
                '        eprintln!("STACK {} {sample} {depth}", case.name);\n        let returned = if status.status == 0 {')
instrumented=original.with_name('controller-stack-worker.rs')
instrumented.write_text(src)
record=q/os.environ.get('MECH_STACK_REPORT', 'layout-active-stack.json')
summary=json.loads(record.read_text()) if record.exists() else {}
run=os.environ.get('MECH_STACK_RUN')
suffix=f'-{run}' if run else ''
for label in sys.argv[1:]:
    p=q/label
    worker=p/('layout-stack-worker'+suffix)
    runtime=p/'runtime.json'
    if not runtime.exists() and (p/'mech-worker-link-receipt.json').exists():
        runtime=p/'mech-worker-link-receipt.json'
    if runtime.exists():
        args=json.loads(runtime.read_text())['link']
        original_worker=p/'mech-worker'
    else:
        args=link
        original_worker=base/'mech-worker'
    argv=[
        item.replace(str(original),str(instrumented)).replace(str(base/'program.o'),str(p/'program.o'))
        .replace(str(original_worker),str(worker))
        for item in args
    ]
    subprocess.run(argv+['-C',f'link-arg={c_object}'],check=True)
    prefix='layout-stack-'+label.replace('/', '-')+suffix
    subprocess.run([sys.executable,'-B',q/'verify-worker.py',worker,p/'metadata.json',prefix],check=True)
    cases={}
    for line in (q/(prefix+'-smokes.stderr')).read_text().splitlines():
        if line.startswith('STACK '):
            _,case,sample,depth=line.split()
            assert int(sample) in (0,1)
            cases.setdefault(case,{})[int(sample)]=int(depth)
    assert len(cases)==81 and all(len(x)==2 and max(x.values())<512*1024 for x in cases.values()),cases
    summary[label]=cases
    peak=max(v for row in cases.values() for v in row.values())
    print(label,'peak native stack bytes touched',peak,'on',
          [case for case,row in cases.items() if peak in row.values()][:12],flush=True)
record.write_text(json.dumps(summary,indent=2)+'\n')
