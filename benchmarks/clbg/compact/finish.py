#!/usr/bin/env python3
"""Audit completed compact measurements and preserve the complete source snapshot."""
import gzip, importlib.util, io, json, tarfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent

def main():
    spec = importlib.util.spec_from_file_location('compact_run', HERE/'run.py')
    run = importlib.util.module_from_spec(spec); spec.loader.exec_module(run)
    path = HERE/'results/apple-m3-compact.json'
    report = json.loads(path.read_text())
    assert report['status'] == 'complete', 'Finish the full-input sweep first'
    studies = json.loads((CLBG/'overview/normalized-results.json').read_text())
    for row in report['programs']:
        row['algorithm'] = run.classify(row)
        assert run.sha(HERE/row['path']) == row['sha256'], row['id']
        if row.get('excluded_full'): continue
        assert len(row['samples_seconds']) >= 3, row['id']
        assert len(row['sample_validation']) == len(row['samples_seconds']), row['id']
        assert all(v['reference_match'] for v in row['sample_validation']), row['id']
        if row['language'] == 'luajit':
            lua = next(r for r in report['programs'] if r['id'] == row['upstream_id'])
            assert lua['sha256'] == row['sha256']
            for metric in ('gzip1_bytes','nonblank_noncomment_lines'):
                assert lua['source'][metric] == row['source'][metric]
    for row in report['julia_existing_audit']:
        bench = row['benchmark']
        manifest = json.loads((CLBG/bench/'manifest.json').read_text())
        old = next(r for r in manifest['programs'] if r['id'] == row['id'])
        assert run.sha(CLBG/bench/old['path']) == row['sha256'], row['id']
        study = next(s for s in studies if s['key'] == bench)
        measured = json.loads((CLBG/study['report']).read_text())
        oldrow = next(r for r in measured['programs'] if r['id'] == row['id'])
        assert len(oldrow['samples_seconds']) >= 3, row['id']
        for metric in ('gzip1_bytes','nonblank_noncomment_lines'):
            assert oldrow['source'][metric] == row['source'][metric], row['id']
        row['preserved_measurement'] = dict(report=study['report'],id=row['id'],
            samples=len(oldrow['samples_seconds']),source_pin_matches=True)
    report['source_policy_audit'] = 'Unchanged listed sources; historical probability subtraction and differing constant/scalar leaf allocation policies labeled. Lua/LuaJIT share identical source pins and sizes. Julia source hashes and complete preserved measurements audited. Timing values are unchanged by this audit.'
    path.write_text(json.dumps(report,indent=2)+'\n')
    files = [p for p in HERE.rglob('*') if p.is_file() and p.suffix not in ('.gz','.png','.sha256') and '__pycache__' not in p.parts]
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer,mode='w') as tar:
        for p in sorted(files): tar.add(p,arcname=str(p.relative_to(CLBG)),recursive=False)
    archive = HERE/'results/apple-m3-compact.sources.tar.gz'
    archive.write_bytes(gzip.compress(buffer.getvalue(),mtime=0))
    checksum = run.sha(archive)
    archive.with_name(archive.name+'.sha256').write_text(checksum+'  '+archive.name+'\n')
    print('Audited complete samples, Lua/LuaJIT source identity, and',len(report['julia_existing_audit']),'preserved Julia sources')
    print('Complete source/timing snapshot SHA-256:',checksum)

if __name__ == '__main__': main()
