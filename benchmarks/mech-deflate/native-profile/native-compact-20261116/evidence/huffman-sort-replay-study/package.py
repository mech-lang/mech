"""Publish completed sorting experiments, including adverse results."""
import argparse
from hashlib import sha256
import gzip
import io
import json
from pathlib import Path
import shutil
import tarfile


def digest(path):
    return sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('qualification_root', type=Path)
    parser.add_argument('matched_builds', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    n, out = args.qualification_root.resolve(), args.output.resolve()
    q, study = n.parent.parent, n/'huffman-sort-replay'
    out.mkdir(parents=True,exist_ok=True)
    read = lambda p:json.loads(Path(p).read_text())
    full = read(study/'hybrid32-full96-results.json')
    assert full['complete'] and full['point_passed']
    builds = read(args.matched_builds/'summary.json')
    replays = read(study/'public-replays/summary.json')
    assert replays['complete'] and len(replays['replays']) == 8
    manifests = {}

    def tree(members, source, prefix):
        assert source.is_dir(), source
        for path in source.rglob('*'):
            if path.is_file() and '__pycache__' not in path.parts:
                members[prefix+'/'+str(path.relative_to(source))] = path

    def archive(name, members):
        pins = {}
        with (out/name).open('wb') as stream, gzip.GzipFile(filename='',fileobj=stream,mode='wb',mtime=0) as compressed:
            with tarfile.open(fileobj=compressed,mode='w') as bundle:
                for relative,path in sorted(members.items()):
                    assert not relative.startswith('/') and '..' not in Path(relative).parts
                    data = path if isinstance(path,bytes) else Path(path).read_bytes()
                    info = tarfile.TarInfo(relative)
                    info.size,info.mode,info.mtime = len(data),0o644,0
                    bundle.addfile(info,io.BytesIO(data))
                    pins[relative] = sha256(data).hexdigest()
        manifests[name] = {'sha256':digest(out/name),'bytes':(out/name).stat().st_size,'members':pins}

    standalone = {}
    for name in ('native-v2','smoke','work','timing'):
        tree(standalone,study/name,'standalone/'+name)
    config = read(study/'config.json')
    standalone['standalone/config.json'] = study/'config.json'
    standalone['standalone/original-C.c'] = Path(config['C_source'])
    for name in config['cases']:
        standalone['standalone/captured-C-inputs/'+name+'.json'] = Path(config['trace_directory'])/(name+'.json')
    standalone['original-program.mec'] = Path(config['codec_source'])
    archive('standalone-replay.tar.gz',standalone)

    proofs = {}
    roles = ['source-sort-shell-control','source-sort-bucket-probe','source-sort-shell-homes',
        'source-sort-bucket-homes','source-sort-shell-packet','source-sort-bucket-packet',
        'source-sort-shell-profiled','source-sort-bucket-profiled','source-sort-hybrid-homes',
        'source-sort-hybrid32-homes','source-sort-hybrid-packet','source-sort-hybrid32-packet',
        'source-sort-hybrid-profiled','source-sort-hybrid32-profiled']
    for role in roles:
        tree(proofs,n/role,role)
    for name in ('linked-input-window-current-source-sort-shell-profiled-source-sort-bucket-profiled',
                 'linked-input-window-current-source-sort-hybrid-profiled-source-sort-hybrid32-profiled'):
        tree(proofs,n/name,name)
    for name in ('public-replays','recovery-packet','recovery-packet-source-sort-hybrid-profiled',
                 'recovery-packet-source-sort-hybrid32-profiled'):
        tree(proofs,study/name,'source-plan/'+name)
    for path in study.glob('*.json'):
        proofs['source-plan/'+path.name] = path
    for path in study.glob('*program.mec'):
        proofs['source-plan/'+path.name] = path
    for path in n.glob('*source-sort*.py'):
        proofs['generation-scripts/'+path.name] = path
    for path in n.glob('source-sort*.log'):
        proofs['generation-logs/'+path.name] = path
    # The newly compiled original-source control is the exact previous image;
    # keep the original checked error/recovery receipts rather than inventing
    # new observations for identical code.
    control = read(n/'source-sort-shell-profiled/bound/runtime.json')
    prior_control = read(n/'source-search-control-profiled/bound/runtime.json')
    assert control['worker_sha256'] == prior_control['worker_sha256']
    tree(proofs,n/'source-search-control-profiled','control-gates/source-search-control-profiled')
    prior_gate = read(n/'source-search-control-profiled/linked-proof.json')
    gate_result = Path(prior_gate['result'])
    tree(proofs,gate_result.parent,'control-gates/linked-proofs')
    proofs['control-gates/reuse.json'] = (json.dumps({'worker_sha256':control['worker_sha256'],
        'actual_new_control_artifacts':read(n/'source-sort-shell-profiled/receipt.json')['artifacts'],
        'previous_control_artifacts':read(n/'source-search-control-profiled/receipt.json')['artifacts'],
        'same_worker':True},indent=2)+'\n').encode()
    archive('full-codec-proofs.tar.gz',proofs)

    runtime = {}
    for prefix in ('source-sort-fiveway','source-sort-hybrid-fiveway','source-sort-hybrid32-full96'):
        for order in ('forward','reverse'):
            name = prefix+'-'+order
            tree(runtime,q/name,'runtime/'+name)
    runtime['runtime/fixtures-original.json'] = q/'fixtures.json'
    runtime['runtime/C-oracle.json'] = q/'pinned-peers/libdeflate-output-oracle.json'
    policy = out.parents[1]/'working-selection-policy.json'
    runtime['runtime/working-selection-policy.json'] = policy
    portable = {}
    for fixture in read(q/'fixtures.json'):
        portable[fixture['name']] = {}
        for tag in ('input','prefix','expected'):
            path = Path(fixture[tag]);pin = digest(path);relative = 'runtime/fixture-data/'+pin+'.bin'
            runtime[relative] = path
            portable[fixture['name']][tag] = {'member':relative,'sha256':pin}
    runtime['runtime/fixtures-portable.json'] = (json.dumps(portable,indent=2)+'\n').encode()
    archive('full-codec-runtime-raw.tar.gz',runtime)

    complete = {}
    tree(complete,args.matched_builds,'builds')
    smoke = study/'deployment-smokes.json'
    assert smoke.exists()
    complete['builds/deployment-smokes.json'] = smoke
    archive('complete-builds.tar.gz',complete)
    summary = {'selected_default_changed':False,'C_next_milestone':64,'C_ultimate_target':81,
        'standalone_build':read(study/'native-v2/build.json'),
        'standalone_work':read(study/'work/results.json'),
        'standalone_timing':read(study/'timing/results.json'),
        'full_bucket_discovery':read(study/'timing-results.json'),
        'full_hybrid_discovery':read(study/'hybrid-timing-results.json'),
        'full_qualification':full,'builds':builds,'public_native_replays':8,
        'control_gate_reuse_same_worker':True,'archives':manifests,
        'scope':'Standalone C-input replay plus actual full-source experiments. The raw bucket overlap regression and every repaired order are retained. No confidence selection gate, no whole-program forecast from replay sums.'}
    (out/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    (out/'archives.json').write_text(json.dumps(manifests,indent=2)+'\n')
    print(json.dumps({k:{a:b for a,b in v.items() if a!='members'} for k,v in manifests.items()},indent=2))


if __name__ == '__main__':
    main()
