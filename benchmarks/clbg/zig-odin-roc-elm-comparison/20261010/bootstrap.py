"""Pin and install native language toolchains and preserve published sources."""
import concurrent.futures
import gzip
import hashlib
import json
import os
from pathlib import Path
import tarfile
import urllib.request

D = Path(__file__).resolve().parent
REPO = 'hanabi1224/Programming-Language-Benchmarks'
COMMIT = 'ad18b203dd1769724f4eea94fc3ac1e99f6593e0'

def request(url):
    return urllib.request.urlopen(urllib.request.Request(url, headers={'User-Agent': 'Mech-benchmark-source-research'}), timeout=120)

def get_json(url):
    with request(url) as response:
        return json.load(response)

def sha(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()

def download(url, path, expected=None):
    if not path.exists():
        temp = path.with_suffix(path.suffix + '.partial')
        with request(url) as response, temp.open('wb') as target:
            while block := response.read(1024 * 1024):
                target.write(block)
        temp.replace(path)
    digest = sha(path)
    if expected:
        assert digest == expected, (path, digest, expected)
    return digest

def install_language(key):
    if key == 'zig':
        index = get_json('https://ziglang.org/download/index.json')
        stable = next((v for k, v in index.items() if k != 'master'))
        asset = stable['aarch64-macos']
        url, expected = asset['tarball'], asset['shasum']
        version = stable['version']
        release = dict(version=version, asset=asset, release_notes=stable.get('notes'))
    else:
        repo, tag = {'odin': ('odin-lang/Odin', 'latest'), 'roc': ('roc-lang/roc', 'tags/alpha4-rolling'), 'elm': ('elm/compiler', 'latest')}[key]
        release = get_json('https://api.github.com/repos/' + repo + '/releases/' + tag)
        match = {'odin': 'macos-arm64', 'roc': 'macos_apple_silicon', 'elm': 'mac-arm.gz'}[key]
        asset = next(a for a in release['assets'] if match in a['name'])
        url, expected = asset['browser_download_url'], None
        if asset.get('digest', '').startswith('sha256:'):
            expected = asset['digest'].split(':', 1)[1]
        version = release['tag_name']
    folder = D / 'support' / key
    folder.mkdir(parents=True, exist_ok=True)
    archive = folder / url.rsplit('/', 1)[1]
    digest = download(url, archive, expected)
    if key == 'elm':
        binary = folder / 'elm'
        if not binary.exists():
            binary.write_bytes(gzip.decompress(archive.read_bytes()))
            binary.chmod(0o755)
    else:
        if not (folder / '.extracted').exists():
            with tarfile.open(archive) as source:
                source.extractall(folder, filter='data')
            (folder / '.extracted').write_text(digest + '\n')
        binaries = [p for p in folder.rglob(key) if p.is_file() and os.access(p, os.X_OK)]
        assert len(binaries) == 1, binaries
        binary = binaries[0]
    record = dict(version=version, path=str(binary), sha256=sha(binary), archive=str(archive), archive_sha256=digest,
        source_url=url, official_checksum_verified=bool(expected), release=release)
    print('INSTALLED', key, version, binary, flush=True)
    return key, record

def preserve_sources():
    tree = json.loads((D / 'upstream/hanabi-tree.json').read_text())['tree']
    wanted = ['nbody', 'binarytrees', 'spectral-norm', 'mandelbrot', 'fannkuch-redux', 'fasta', 'knucleotide', 'revcomp', 'regexredux', 'pidigits']
    sources = [r['path'] for r in tree if r['type'] == 'blob' and r['path'].endswith(('.zig', '.odin'))
        and any('/' + benchmark + '/' in r['path'] for benchmark in wanted)]
    sources += ['LICENSE', 'bench/bench_zig.yaml', 'bench/bench_odin.yaml']
    records = []
    for relative in sources:
        path = D / 'upstream/hanabi' / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        url = 'https://raw.githubusercontent.com/' + REPO + '/' + COMMIT + '/' + relative
        digest = download(url, path)
        records.append(dict(path=str(path), upstream_path=relative, sha256=digest,
            source_url='https://github.com/' + REPO + '/blob/' + COMMIT + '/' + relative))
    return records

def main():
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        runtimes = dict(pool.map(install_language, ['zig', 'odin', 'roc', 'elm']))
    manifest = dict(repository=REPO, commit=COMMIT, runtimes=runtimes, sources=preserve_sources())
    (D / 'bootstrap.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print('SOURCE POOL', len(manifest['sources']) - 3, flush=True)

if __name__ == '__main__':
    main()
