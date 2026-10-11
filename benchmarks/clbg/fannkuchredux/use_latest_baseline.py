#!/usr/bin/env python3
"""Select the latest preserved pre-fannkuch Mandelbrot/k-nucleotide baselines."""
import json
from pathlib import Path
import run

path=run.BUILD/'previous-regressions/prepared.json'
rows=json.loads(path.read_text())
original=path.with_name('original-report-baseline.json')
if not original.exists(): original.write_text(path.read_text())
report=run.HERE.parent/'revcomp/results/runtime-regression.json'
previous=json.loads(report.read_text())['programs']
for row in rows:
    if not row['id'].endswith('previous'): continue
    key=row['id'][:-len('previous')]+'current'
    old=next(r for r in previous if r['id']==key)
    command=[str((run.ROOT/old['command'][0]).resolve())]
    assert run.sha(command[0])==old['executable_sha256'],key
    assert row['source']['sha256']==old['source']['sha256'],key
    row.update(command=command,executable_sha256=old['executable_sha256'],
        native_object_sha256=old['native_object_sha256'],
        previous_report=str(report.relative_to(run.ROOT)))
for row in rows:
    if not row['id'].endswith('current'): continue
    old=next(r for r in rows if r['id']==row['id'][:-len('current')]+'previous')
    row['same_native_object_as_previous']=row['native_object_sha256']==old['native_object_sha256']
path.write_text(json.dumps(rows,indent=2)+'\n')
