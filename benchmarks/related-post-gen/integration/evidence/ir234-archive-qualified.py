from pathlib import Path
import gzip, hashlib, json, os, shutil, time

R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
D = R/'results/native-deflate-exhaustion-proof-integration-20261106'
D.mkdir(exist_ok=False)
objects = D/'objects'
objects.mkdir()
prefix = '/private/tmp/mech-related-deflate-exhaustion-proof-integration-'
roots = {
    'compiler-build': Path(prefix+'opt-20261106-v1'),
    'gpu-tests': Path(prefix+'tests-20261106-v1'),
    'runners': Path(prefix+'runners-20261106-v1'),
    'controls': Path(prefix+'regressions-20261106-v1'),
    'runtime-only': Path(prefix+'runtime-check-20261106-v1'),
    'feature-check': Path(prefix+'feature-check-20261106-v1'),
    'old-objects': Path(prefix+'old-objects-20261106-v1'),
    'source-ordered-source': Path('/private/tmp/mech-related-ordered-subsets-source-20261106-v1'),
    'source-ordered-latency': Path('/private/tmp/mech-related-ordered-subsets-source-latency-20261106-v1'),
    'source-ordered-resources': Path('/private/tmp/mech-related-ordered-subsets-source-resources-20261106-v1'),
    'source-fused-source': Path('/private/tmp/mech-related-ordered-subsets-fused-source-20261106-v1'),
    'source-fused-latency': Path('/private/tmp/mech-related-ordered-subsets-fused-source-latency-20261106-v1'),
    'source-fused-resources': Path('/private/tmp/mech-related-ordered-subsets-fused-source-resources-20261106-v1'),
    'source-reused-masks-source': Path('/private/tmp/mech-related-ordered-subsets-reused-masks-source-20261106-v1'),
    'source-reused-masks-latency': Path('/private/tmp/mech-related-ordered-subsets-reused-masks-source-latency-20261106-v1'),
    'source-reused-masks-resources': Path('/private/tmp/mech-related-ordered-subsets-reused-masks-source-resources-20261106-v1'),
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
             compiler='c4649860a5e445c1ce1bdec8503870e8a455da17',
             identity='IR234/work115/ABI41/image9', root=str(D),
             raw_archives_remotely_uploaded=False, performance_cohort_run=True,
             files=files, symlinks=links, verified_files=len(files),
             unique_blobs=len(list(objects.glob('*.gz'))),
             seconds=time.monotonic()-started)
(D/'archive-index.json').write_text(json.dumps(index, indent=2)+'\n')
print(json.dumps(dict(status='verified', files=len(files), unique_blobs=index['unique_blobs'])), flush=True)
