"""Correctness and performance probes, kept outside formal timing samples."""
import argparse,hashlib,json
from datetime import datetime,timezone
from pathlib import Path
from run import execute,compact,REQUIRED
D=Path(__file__).resolve().parent
ap=argparse.ArgumentParser(description=__doc__)
ap.add_argument('--files',nargs='+')
ap.add_argument('--full',action='store_true')
ap.add_argument('--output',default='preflight.json')
args=ap.parse_args()
path=D/'results'/args.output
report=json.loads(path.read_text()) if path.exists() else []
for built in json.loads((D/'results/builds.json').read_text()):
    source=D/built['source']
    if args.files and source.name not in args.files:continue
    bench=source.stem.split('-')[0]
    row=dict(key=bench+':'+built['language']+'-'+source.stem,benchmark=bench,language=built['language'],command=[built['binary']]+([] if bench in ('knucleotide','revcomp','regexredux') else ['{n}']))
    compact.wait_for_environment(REQUIRED)
    print('PROBE',row['key'],'full' if args.full else 'small',flush=True)
    result=execute(row,args.full,14400)
    assert not result['competing_compilers'] and not result['power_changes'],result
    report.append(dict(key=row['key'],source_sha256=built['source_sha256'],binary_sha256=built['binary_sha256'],full=args.full,result=result,at=datetime.now(timezone.utc).isoformat(),formal_sample=False))
    path.write_text(json.dumps(report,indent=2)+'\n')
    print('PASS',row['key'],f'{result["seconds"]:.6f}s',flush=True)
