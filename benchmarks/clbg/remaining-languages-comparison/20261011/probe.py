"""Shared unchanged-workload output validation and guarded execution."""
import importlib.util
from pathlib import Path
from dotnet_fetch import D,C,sha

spec=importlib.util.spec_from_file_location('array_runner',C/'array-comparison/all-20261009/run.py')
arrays=importlib.util.module_from_spec(spec);spec.loader.exec_module(arrays)
compact=arrays.compact
compact.COMPILERS.update(['gnat1','fpc','ppca64','gfortran','cc1','f951','dart','toitc','dotnet'])
WORK,REQUIRED=arrays.WORK,arrays.REQUIRED

def execute(row,full=False,timeout=14400):
    # The timed .NET process is itself a JIT runtime, not a competing compiler.
    # All preparations stop before any samples are admitted.
    compact.COMPILERS.discard('dotnet')
    compact.COMPILERS.discard('dart')
    # Execution is strictly sequential. Respect image-based runtimes' working
    # directories without changing any arguments or the measured boundary.
    old_root=compact.ROOT
    compact.ROOT=Path(row.get('cwd',old_root))
    try:
        if row['benchmark']=='nbody':
            return compact.execute(row,full,timeout,watch_compilers=True,expected_power=REQUIRED)
        return arrays.execute(row,full,timeout)
    finally:
        compact.ROOT=old_root

def save(path,value):
    import json
    tmp=path.with_suffix('.json.tmp');tmp.write_text(json.dumps(value,indent=2)+'\n');tmp.replace(path)

def validate_small(row):
    # This is an untimed preparation check; no measurements are admitted here.
    before=compact.COMPILERS.copy();compact.COMPILERS.clear()
    try:return execute(row,False,120)['validation']
    finally:compact.COMPILERS.update(before)
