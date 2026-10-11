#!/usr/bin/env python3
"""Audit production/source fixtures while excluding generated build projects."""
import shutil,subprocess,tempfile
from pathlib import Path
import run
with tempfile.TemporaryDirectory(prefix='mech-control-audit-') as tmp:
 target=Path(tmp)
 allowlist=run.ROOT/'scripts/unsafe-boundaries.allowlist'
 files=set()
 for line in allowlist.read_text().splitlines():
  if line and not line.startswith('#'):files.add(run.ROOT/line.split('|')[0])
 paths=subprocess.check_output(['rg','--files','--hidden','-g','*.rs','-g','!**/target/**','-g','!**/.build/**','-g','!**/*.project/**','-g','!**/vendor/**','src','hosts','tests','benchmarks/clbg/fasta'],cwd=run.ROOT,text=True).splitlines()
 files.update(run.ROOT/path for path in paths)
 files.add(allowlist)
 for path in files:
  dest=target/path.relative_to(run.ROOT);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(path,dest)
 subprocess.run([str(run.ROOT/'scripts/check-unsafe-boundaries.sh')],env=__import__('os').environ|{'UNSAFE_BOUNDARY_ROOT':str(target)},check=True)
