"""Source-only candidate: pin manual V allocation review, then seal tenth study."""
import argparse
import fcntl
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import traceback
from types import SimpleNamespace

PACKET = Path(__file__).resolve().parent
HERE = PACKET.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
LOWERING = HERE / 'lowering-evidence/builder-v3-work1/report.json'
WORK = HERE / 'work-qualification/builder-v3-work1/binarytrees/report.json'
SOURCE_REVIEW = PACKET / 'source-review.json'
APPROVAL = PACKET / 'qualified-allocation-review.json'
OUTCOME = PACKET / 'controller-outcome.json'


def sha(path):
    with Path(path).open('rb') as stream: return hashlib.file_digest(stream, 'sha256').hexdigest()


def new_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream: json.dump(value, stream, indent=2); stream.write('\n')


def verify(pins):
    for path, digest in pins.items(): assert sha(path) == digest, ('Reviewed actual artifact changed', path)


def module(name, path):
    assert name not in sys.modules
    spec = importlib.util.spec_from_file_location(name, path); value = importlib.util.module_from_spec(spec)
    sys.modules[name] = value; spec.loader.exec_module(value)
    assert Path(value.__file__).resolve() == path.resolve()
    return value


def seal():
    assert not OUTCOME.exists() and not APPROVAL.exists(), 'Exclusive additive review; preserve failures without retry'
    result = dict(status='failed', native_benchmark_processes=0, compiler_processes=0, chart_admission=False)
    try:
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            lowering = json.loads(LOWERING.read_text()); work = json.loads(WORK.read_text())
            manual = json.loads(SOURCE_REVIEW.read_text())
            assert manual['status'] == 'reviewed-source-meaning-awaiting-guarded-identity-seal'
            assert manual['source_revision'] == 'production-builder-v3' and manual['unresolved'] == []
            assert len(manual['profiles']) == 2 and {row['variant'] for row in manual['profiles']} == {'compact', 'optimized'}
            assert lowering['status'] == 'generated-C-retained-awaiting-root-allocator-review'
            assert lowering['source_revision'] == 'production-builder-v3' and lowering['pins_unchanged_at_end'] is True
            assert lowering['native_benchmark_runs'] == lowering['full_performance_runs'] == 0
            assert work['status'] == 'passed-real-node-and-retention-traces' and work['pins_unchanged_at_end'] is True
            verify(lowering['pins']); verify(work['pins'])
            pins = dict(lowering['pins'])
            for path, digest in work['pins'].items():
                assert path not in pins or pins[path] == digest; pins[path] = digest
            inputs = [LOWERING, WORK, SOURCE_REVIEW, Path(__file__).resolve(), PACKET / 'REVIEW.md']
            pins.update({str(path): sha(path) for path in inputs})
            profiles = []
            for review in manual['profiles']:
                produced = next(row for row in lowering['profiles'] if row['id'] == review['id'])
                assert produced['variant'] == review['variant'] and produced['source_sha256'] == review['source_sha256']
                assert len(produced['generated_c']) == 1
                generated = Path(produced['generated_c'][0])
                assert pins[str(generated)] == review['generated_c_sha256'] == sha(generated)
                text = generated.read_text()
                assert '#define CUSTOM_DEFINE_gcboehm\n' in text and '#define CUSTOM_DEFINE_vgc\n' not in text
                assert '#define HEAP(type, expr) ((type*)builtin__memdup((void*)&((type[]){expr}[0]), sizeof(type)))' in text
                creator = text.split('VV_LOC main__Node* main__create(int depth) {', 1)[1].split('\n}', 1)[0]
                assert 'return (HEAP(main__Node, ((main__Node){.left = ((void*)0),.right = ((void*)0),})));' in creator
                assert creator.count('main__create(depth - 1)') == 2
                assert 'builtin___v_malloc(sizeof(main__Node) == 0 ? 1 : sizeof(main__Node))' in creator
                assert creator.index('builtin___v_malloc') < creator.index('_t3->left = main__create') < creator.index('_t3->right = main__create')
                assert 'voidptr builtin__memdup(voidptr src, isize sz) {' in text and 'u8* mem = builtin___v_malloc(sz);' in text
                assert 'res = GC_MALLOC(n);' in text and 'main__Node* left;\n\tmain__Node* right;' in text
                assert review['both_recursive_children'] is review['interior_and_leaf_heap_allocations'] is True
                profiles.append(dict(id=review['id'], variant=review['variant'], source_sha256=review['source_sha256'],
                    status='passed', both_recursive_children=True, interior_and_leaf_heap_allocations=True,
                    creator_proof=dict(path=str(generated), sha256=review['generated_c_sha256'], symbol=review['creator_symbol']),
                    leaf_allocator_proof=dict(path=str(generated), sha256=review['generated_c_sha256'], symbol=review['leaf_allocator_symbol']),
                    actual_allocator_proof=dict(path=str(generated), sha256=review['generated_c_sha256'], symbol=review['native_allocator_symbol']),
                    semantic_review=review['review']))
            # Import only the immutable producer's actual source bytes after
            # their retained evidence pins have passed under the same mutex.
            for name in ('full_study.py', 'prepare.py', 'prepare_builder_v3.py'):
                path = HERE / name
                assert str(path) in pins and sha(path) == pins[str(path)]
            sealer_path = HERE / 'seal_builder_v3_study.py'
            # This reviewed metadata consumer was not an input of the earlier
            # lowering producer. Capture its current source pin explicitly,
            # without claiming it existed in that old receipt's pin map.
            pins[str(sealer_path)] = sha(sealer_path)
            sys.dont_write_bytecode = True; sys.path.insert(0, str(HERE))
            full = module('full_study', HERE / 'full_study.py')
            sealer = module('v_tree_existing_exact_sealer', sealer_path)
            qualification = HERE / 'qualification/production-builder-v3/binarytrees/probes.json'
            qualified = json.loads(qualification.read_text())
            sealer.validate_work_coverage(work, qualified['profiles'], 'binarytrees', 'production-builder-v3')
            review = dict(status='passed', review_identity=manual['review_identity'], source_revision='production-builder-v3',
                pins_unchanged_at_end=True, both_recursive_children=True, interior_and_leaf_heap_allocations=True,
                lowering_evidence=dict(path=str(LOWERING), sha256=pins[str(LOWERING)]), profiles=profiles, pins=pins,
                source_semantic_review=dict(path=str(SOURCE_REVIEW), sha256=pins[str(SOURCE_REVIEW)]),
                generated_evidence_binary_executed=False, observed_graph_traces=str(WORK),
                native_benchmark_processes=0, full_performance_runs=0, chart_admission=False, canonical_LOC_admitted=False)
            sealer.validate_allocation_approval(review, qualified['profiles'], 'production-builder-v3', full.configure('binarytrees'))
            verify(pins); new_json(APPROVAL, review)
            approval_sha, work_sha = sha(APPROVAL), pins[str(WORK)]
        # The unchanged sealer acquires its own common descriptor. Do not hold
        # this earlier descriptor while invoking that lock-using operation.
        sealer.seal(SimpleNamespace(benchmark='binarytrees', source_revision='production-builder-v3',
            work_report=str(WORK), work_report_sha256=work_sha, allocation_review=str(APPROVAL), allocation_review_sha256=approval_sha))
        result.update(status='sealed-awaiting-root-manifest-review', allocation_review=str(APPROVAL), allocation_review_sha256=approval_sha)
    except BaseException as error:
        result.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            (PACKET / 'actual-error.log').write_text(traceback.format_exc())
        raise
    finally:
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            files = [path for path in PACKET.rglob('*') if path.is_file() and path != OUTCOME]
            manifest = HERE / 'studies/binarytrees/full-study/manifest.json'
            if manifest.exists():
                result['manifest'] = str(manifest); result['manifest_sha256'] = sha(manifest); files.append(manifest)
            result['actual_terminal_evidence_pins'] = {str(path): sha(path) for path in files}
            new_json(OUTCOME, result)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('--seal', action='store_true', required=True); parser.parse_args()
    print('WAIT common mutex: exact V tree allocation review and tenth workload manifest', flush=True)
    with (PACKET / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB); seal()
