from pathlib import Path
import gzip, hashlib, json, os, shutil, time

R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
D = R/'results/native-boolean-implication-freshness-20261106'
D.mkdir(exist_ok=False)
objects = D/'objects'
objects.mkdir()
roots = {
    'compiler-build': Path('/private/tmp/mech-related-native-boolean-implication-freshness-opt-20261106-v7'),
    'gpu-tests': Path('/private/tmp/mech-related-native-boolean-implication-freshness-tests-20261106-v4'),
    'runners': Path('/private/tmp/mech-related-native-boolean-implication-freshness-runners-20261106-v4'),
    'controls': Path('/private/tmp/mech-related-native-boolean-implication-freshness-regressions-20261106-v2'),
    'runtime-only': Path('/private/tmp/mech-related-native-boolean-implication-freshness-runtime-check-20261106-v2'),
    'feature-check': Path('/private/tmp/mech-related-native-boolean-implication-freshness-feature-check-20261106-v2'),
    'old-objects': Path('/private/tmp/mech-related-native-boolean-implication-freshness-old-objects-20261106-v2'),
    'profiles': Path('/private/tmp/mech-related-native-boolean-implication-mask-reuse-pc-profile-20261106-v1'),
    'latency': Path('/private/tmp/mech-related-native-boolean-implication-mask-reuse-latency-20261106-v1'),
    'resources': Path('/private/tmp/mech-related-native-boolean-implication-mask-reuse-resources-20261106-v1'),
    'initial-probe': Path('/private/tmp/mech-related-native-boolean-implication-freshness-probe-20261106-v1'),
    'initial-tests-1': Path('/private/tmp/mech-related-native-boolean-implication-freshness-tests-20261106-v1'),
    'initial-tests-2': Path('/private/tmp/mech-related-native-boolean-implication-freshness-tests-20261106-v2'),
    'initial-tests-3': Path('/private/tmp/mech-related-native-boolean-implication-freshness-tests-20261106-v3'),
}
files = {}
links = {}
started = time.monotonic()
for label, root in roots.items():
    for source in sorted(root.rglob('*')):
        name = label+'/'+str(source.relative_to(root))
        if source.is_symlink():
            links[name] = str(source.readlink())
            continue
        if not source.is_file():
            continue
        raw = source.read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        blob = objects/(digest+'.gz')
        if not blob.exists():
            with blob.open('wb') as out:
                out.write(gzip.compress(raw, compresslevel=6, mtime=0))
                out.flush()
                os.fsync(out.fileno())
        assert hashlib.sha256(gzip.decompress(blob.read_bytes())).hexdigest() == digest
        files[name] = dict(source=str(source), sha256=digest, bytes=len(raw),
                           mode=source.stat().st_mode & 0o777,
                           blob=str(blob.relative_to(D)),
                           compressed_bytes=blob.stat().st_size)
    print(label, 'archived and SHA-verified', flush=True)
index = dict(schema='mech.related-post-gen.qualified-compiler-archive.v1',
             compiler='cbe5b47839b4fe2300fe16ebc1fcacfcfa03f98b',
             identity='IR235/work115/ABI41/image9', root=str(D),
             raw_archives_remotely_uploaded=False, performance_cohort_run=True,
             files=files, symlinks=links, verified_files=len(files),
             unique_blobs=len(list(objects.glob('*.gz'))),
             seconds=time.monotonic()-started)
manifest=R/'results/archived-superseded-executables-20261106/manifest.jsonl'
external=[]
for line in manifest.read_text().splitlines():
    row=json.loads(line)
    if '/mech-related-native-boolean-implication-freshness-' in row.get('original_path',''):
        blob=Path(row['blob']);assert hashlib.sha256(gzip.decompress(blob.read_bytes())).hexdigest()==row['sha256'];external.append(row)
index['losslessly_retained_superseded_executables']=external
(D/'archive-index.json').write_text(json.dumps(index, indent=2)+'\n')
print(json.dumps(dict(status='verified', files=len(files), unique_blobs=index['unique_blobs'])), flush=True)
