"""Bind one deferred historical preparation import; original six fixtures reused."""
import argparse
import fcntl
import hashlib
import importlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import traceback
from types import SimpleNamespace

PACKET = Path(__file__).resolve().parent
CLBG = PACKET.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
ORIGINAL = CLBG/'loc-normalization/20261012/nim-source-replay-v1'
ENTRY = PACKET.parent/'source20-entry-repairs-v1/replay.py'
ENTRY_SHA = 'df7b19f86474a18591c757d0e474a67f1da0e1cbd5a0f90689839c09fbded14d'
RUNNER_SHA = '19a38d8aa24d1f0ebbdf0d7e4f5033345eb7db8e58bd11ea42b15063b1934cda'
FIXTURE = ORIGINAL/'jobs/fixtures-v1/fixture-receipt.json'
FIXTURE_SHA = 'aec967282855da29084de8d445b6f8311c42f3c1be5963f162078e1f167e2e52'
FAILED = ORIGINAL/'jobs/source20-repaired-v3'
STOPPED_SHA = 'edda317536087182ca63ca543d007feb7610c5105419aa63657f940f31516f31'
ERROR_SHA = '3d67dc7795dd4bc963d7e2a386174e6288f5e5d009bc03471b9beaf48ce8c970'
NIM = CLBG/'nim-comparison/20261012'


def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path,value):
    with Path(path).open('x') as stream:
        json.dump(value,stream,indent=2); stream.write('\n'); stream.flush(); os.fsync(stream.fileno())


def pin(path,pins,expected=None):
    path=Path(path).resolve(strict=True); value=sha(path)
    assert expected is None or expected==value, ('Changed source/receipt',str(path))
    assert str(path) not in pins or pins[str(path)]==value
    pins[str(path)]=value
    return value


def verify(pins):
    for path,digest in pins.items(): assert sha(path)==digest, ('Changed pinned input',path)


def fresh(path,name,pins):
    path=Path(path).resolve(strict=True); assert name not in sys.modules
    assert sha(path)==pins[str(path)]; source=path.read_bytes()
    assert hashlib.sha256(source).hexdigest()==pins[str(path)]
    module=importlib.util.module_from_spec(importlib.util.spec_from_file_location(name,path))
    sys.modules[name]=module; exec(compile(source,str(path),'exec'),module.__dict__)
    assert sys.modules[name] is module and Path(module.__file__).resolve()==path
    assert sha(path)==pins[str(path)]
    return module


def main():
    parser=argparse.ArgumentParser(); parser.add_argument('--revision',required=True)
    parser.add_argument('--failed-terminal-sha256',required=True); args=parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}',args.revision)
    assert args.revision not in ('fixtures-v1','source20-restored-v2','source20-repaired-v3')
    assert re.fullmatch(r'[a-f0-9]{64}',args.failed_terminal_sha256)
    output=ORIGINAL/'jobs'/args.revision
    preflight=PACKET/'preflights'/args.revision
    print('WAIT common mutex: exact Nim deferred preparation import binding',flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX); held,named=os.fstat(lock.fileno()),LOCK.stat()
        assert (held.st_dev,held.st_ino)==(named.st_dev,named.st_ino)
        assert not output.exists()
        preflight.mkdir(parents=True)
        pins={}; record=dict(status='failed-entry-preflight',benchmark_processes_started=0,
            compiler_commands_started=0,new_fixture_runs=0,actual_fixture_methods_reused=6,
            common_mutex=str(LOCK),exclusive_common_timing_mutex_held=True,
            automatic_retry=False,canonical_LOC_admitted=False)
        try:
            for path in PACKET.iterdir():
                if path.is_file(): pin(path,pins)
            pin(sys.executable,pins); pin(ENTRY,pins,ENTRY_SHA)
            pin(ORIGINAL/'qualify_replay.py',pins,RUNNER_SHA); pin(FIXTURE,pins,FIXTURE_SHA)
            terminal_path=FAILED/'terminal-pins.json'
            pin(terminal_path,pins,args.failed_terminal_sha256)
            terminal=json.loads(terminal_path.read_text())
            assert terminal['status']=='failed-stop'
            for group in ('pins','prerequisite_pins'):
                for path,digest in terminal[group].items(): pin(path,pins,digest)
            pin(FAILED/'stopped-report.json',pins,STOPPED_SHA); pin(FAILED/'exception.log',pins,ERROR_SHA)
            stopped=json.loads((FAILED/'stopped-report.json').read_text())
            assert stopped['status']=='stopped-unqualified' and stopped['error_type']=='ModuleNotFoundError'
            assert stopped['error']=="No module named 'guarded_prepare_v5'"
            assert type(stopped['compiler_commands_started']) is int and stopped['compiler_commands_started']==0
            failure=(FAILED/'exception.log').read_text()
            assert 'proof_reuse_v5.py", line 214, in validate_prior_reports' in failure
            assert 'import guarded_prepare_v5 as preparation' in failure
            prior=fresh(ENTRY,'preserved_nim_source20_entry_repair_v1',pins)
            original=fresh(ORIGINAL/'qualify_replay.py','preserved_nim_source20_count_runner',pins)
            prior.repaired_nim(original,pins,output)
            original_import=importlib.import_module

            def bound_import(name):
                value=original_import(name)
                assert name=='qualify_work'
                assert Path(value.__file__).resolve()==original.QUALIFIER/'qualify_work.py'
                # The old qualifier imports insert the cohort search path;
                # its caller then pops that path. Bind the one deferred import
                # while its exact prerequisite bytes are independently known.
                qualification=json.loads(original.QUALIFICATION.read_text())
                assert qualification['proposal_sha256']==original.PROPOSAL_SHA
                preparation_path=NIM/'guarded_prepare_v5.py'
                expected=qualification['pins'][str(preparation_path)]
                pin(preparation_path,pins,expected)
                fresh(preparation_path,'guarded_prepare_v5',pins)
                verify(pins)
                return value

            # Proxy only this freshly loaded prior entry module. No global
            # importlib/source file/counter/qualification function is changed.
            prior.importlib=SimpleNamespace(util=importlib.util,import_module=bound_import)
            record.update(status='exact-deferred-preparation-binding-ready',
                prior_entry=str(ENTRY),prior_entry_sha256=ENTRY_SHA,
                original_count_runner=str(ORIGINAL/'qualify_replay.py'),
                failed_revision=str(FAILED),failed_terminal_sha256=args.failed_terminal_sha256,
                deferred_module='guarded_prepare_v5',
                deferred_source=str(NIM/'guarded_prepare_v5.py'),
                original_fixtures_sources_grammar_counter_and_qualified_validator_changed=False)
            verify(pins)
        except BaseException as error:
            record.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
            with (preflight/'failure.log').open('x') as stream: stream.write(traceback.format_exc())
            raise
        finally:
            available={}; failures=[]; candidates=dict(pins)
            for path in preflight.rglob('*'):
                if path.is_file(): candidates.setdefault(str(path.resolve()),None)
            for path,expected in candidates.items():
                try:
                    value=sha(path); assert expected is None or value==expected; available[path]=value
                except BaseException as error: failures.append(dict(path=path,error=str(error) or type(error).__name__))
            if failures: record.update(status='failed-terminal-integrity',pin_failures=failures)
            record['pins']=available; save(preflight/'report.json',record)
            if failures: raise RuntimeError('Deferred entry preflight integrity failed; partial pins retained')
        pin(preflight/'report.json',pins); verify(pins)
    # The original frozen runner reacquires its own actual common mutex, then
    # rechecks all repair/prerequisite pins before the adapted entry or any
    # frontend/count command. Never hold this descriptor while it reacquires.
    original.run(SimpleNamespace(revision=args.revision,fixtures_only=False,
        fixture_receipt=FIXTURE,fixture_sha256=FIXTURE_SHA))


if __name__=='__main__': main()
