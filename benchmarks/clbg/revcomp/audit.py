#!/usr/bin/env python3
"""Audit production/source fixtures while excluding generated build projects."""
import shutil,subprocess,tempfile
from pathlib import Path
import run
with tempfile.TemporaryDirectory(prefix='mech-byte-audit-') as tmp:
 target=Path(tmp)
 allowlist=run.ROOT/'scripts/unsafe-boundaries.allowlist'
 files=set()
 for line in allowlist.read_text().splitlines():
  if line and not line.startswith('#'):files.add(run.ROOT/line.split('|')[0])
 for folder in ('src','hosts','tests','benchmarks/clbg/revcomp'):
  for path in (run.ROOT/folder).rglob('*.rs'):
   if any(part in ('.build','target','vendor') or part.endswith('.project') for part in path.parts):continue
   files.add(path)
 files.add(allowlist)
 for path in files:
  dest=target/path.relative_to(run.ROOT);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(path,dest)
 subprocess.run([str(run.ROOT/'scripts/check-unsafe-boundaries.sh')],env=__import__('os').environ|{'UNSAFE_BOUNDARY_ROOT':str(target)},check=True)
