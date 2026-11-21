"""Archive the conditional adaptive-block experiment and every measured outcome."""
import argparse
from hashlib import sha256
import gzip
import io
import json
from pathlib import Path
import tarfile


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('qualification_root', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    n, out = args.qualification_root.resolve(), args.output.resolve()
    study, q, builds = n/'lazy-block-check', n.parent.parent, n/'lazy-block-matched-builds'
    read = lambda p:json.loads(Path(p).read_text())
    digest = lambda p:sha256(Path(p).read_bytes()).hexdigest()
    manifests = {}

    def tree(members, source, prefix):
        for path in source.rglob('*'):
            if path.is_file() and '__pycache__' not in path.parts and path.name != 'compile_profile':
                members[prefix+'/'+str(path.relative_to(source))] = path

    def archive(name, members):
        pins = {}
        with (out/name).open('wb') as stream, gzip.GzipFile(filename='', fileobj=stream, mode='wb', mtime=0) as zipped:
            with tarfile.open(fileobj=zipped, mode='w') as tar:
                for relative, path in sorted(members.items()):
                    assert not relative.startswith('/') and '..' not in Path(relative).parts
                    data = path if isinstance(path, bytes) else path.read_bytes()
                    info = tarfile.TarInfo(relative)
                    info.size, info.mode, info.mtime = len(data), 0o644, 0
                    tar.addfile(info, io.BytesIO(data))
                    pins[relative] = sha256(data).hexdigest()
        manifests[name] = {'sha256':digest(out/name), 'bytes':(out/name).stat().st_size, 'members':pins}

    native = {}
    tree(native, study, 'study')
    held = n/'source-sort-hybrid32-profiled'
    for directory in ['frontend','bound','collector-final-counts-475','rebase-counts-362']:
        tree(native, held/directory, 'held-source/'+directory)
    native['held-source/receipt.json'] = held/'receipt.json'
    for path in [n/name for name in ['prepare-lazy-block-check.py','profile-lazy-block-check.py','qualify-lazy-block-check.py','time-lazy-block-check.py','measure-lazy-block-builds.py','count-final-lazy.py']]:
        native['generation-scripts/'+path.name] = path
    native['generation-scripts/count-current-rebase.py'] = n/'count-current-rebase.py'
    native['comparison-sources/main.c'] = q/'minimal-deployment/source/main.c'
    config = read(n/'huffman-sort-replay/config.json')
    c = Path(config['C_source'])
    for name in ['bt_matchfinder.h','matchfinder_common.h']:
        native['comparison-sources/'+name] = c.parent/name
    archive('native-proofs.tar.gz', native)

    runtime = {}
    for prefix in ['lazy-block-check']:
        for order in ['forward','reverse']:
            name = prefix+'-'+order
            tree(runtime, q/name, 'runtime/'+name)
    runtime['runtime/fixtures-original.json'] = q/'fixtures.json'
    runtime['runtime/C-oracle.json'] = q/'pinned-peers/libdeflate-output-oracle.json'
    portable = {}
    for fixture in read(q/'fixtures.json'):
        portable[fixture['name']] = {}
        for tag in ['input','prefix','expected']:
            path = Path(fixture[tag]); pin = digest(path)
            relative = 'runtime/fixture-data/'+pin+'.bin'
            runtime[relative] = path
            portable[fixture['name']][tag] = {'member':relative, 'sha256':pin}
    runtime['runtime/fixtures-portable.json'] = (json.dumps(portable,indent=2)+'\n').encode()
    archive('runtime-raw.tar.gz', runtime)

    complete = {}
    tree(complete, builds, 'builds')
    archive('complete-builds.tar.gz', complete)
    summary = {'selected_default_changed':False, 'selected_Roc_wins':81, 'selected_C_wins':63,
               'next_C_target':64, 'ultimate_C_target':81,
               'conditional_check':read(study/'timing-results.json'),
               'builds':read(builds/'summary.json'), 'public_native_replays':2,
               'candidate_promoted':False, 'candidate_full81_qualified':False,
               'confidence_assessed':False, 'archives':manifests,
               'scope':'Equivalent pure conditional adaptive-block predicates. Both complete21case24block orders retain all observed named wins, with mixed small speed changes and no64thwin. Full81 qualification notrun. Original-source default and earlier HF-U32 alternative unchanged.'}
    (out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    (out/'archives.json').write_text(json.dumps(manifests,indent=2)+'\n')
    print(json.dumps({k:{a:b for a,b in v.items() if a!='members'} for k,v in manifests.items()},indent=2))


if __name__ == '__main__':
    main()
