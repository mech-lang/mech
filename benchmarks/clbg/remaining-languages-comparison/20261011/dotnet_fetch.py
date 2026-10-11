"""Preserve all officially listed .NET benchmark sources and install pinned SDK."""
import concurrent.futures
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tarfile
from urllib.request import Request, urlopen

D = Path(__file__).resolve().parent
C = D.parents[1]
BASE = 'https://benchmarksgame-team.pages.debian.net/benchmarksgame/'
LANGS = {'csharpcore': ('csharp', '.cs'), 'fsharpcore': ('fsharp', '.fs')}
spec = importlib.util.spec_from_file_location('official_parser', C / 'nbody/fetch_sources.py')
parser = importlib.util.module_from_spec(spec)
spec.loader.exec_module(parser)

def sha(path, algo='sha256'):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, algo).hexdigest()

def request(url):
    return urlopen(Request(url, headers={'User-Agent': 'Mech benchmark source audit'}), timeout=120)

def fetch(item):
    upstream, ident = item
    language, suffix = LANGS[upstream]
    url = BASE + 'program/' + ident + '.html'
    page = D / 'upstream/dotnet' / (ident + '.html')
    if not page.exists():
        page.write_bytes(request(url).read())
    pre = parser.Preformatted()
    pre.feed(page.read_text())
    assert pre.blocks, ident
    original = page.with_suffix(suffix)
    original.write_text(pre.blocks[0].strip() + '\n')
    target = D / 'sources' / language / original.name
    if not target.exists():
        target.write_bytes(original.read_bytes())
    return dict(id=ident, key=ident.split('-'+upstream+'-')[0]+':'+ident,
        benchmark=ident.split('-'+upstream+'-')[0], language=language,
        source=str(target), source_sha256=sha(target), original_source=str(original),
        original_source_sha256=sha(original), source_url=url,
        upstream_notes=pre.blocks[1].strip() if len(pre.blocks)>1 else '',
        adaptations=[], local_adaptation=False)

def sources():
    (D / 'upstream/dotnet').mkdir(parents=True, exist_ok=True)
    for lang,_ in LANGS.values():
        (D / 'sources' / lang).mkdir(exist_ok=True)
    items, tables = [], {}
    for upstream in LANGS:
        url = BASE + 'measurements/' + upstream + '.html'
        p = D / 'upstream/dotnet' / (upstream+'-measurements.html')
        if not p.exists():
            p.write_bytes(request(url).read())
        tables[upstream] = dict(url=url, sha256=sha(p))
        items += [(upstream,x) for x in sorted(set(re.findall(r'program/(\w+-'+upstream+r'-\d+)\.html',p.read_text())))]
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        rows = list(pool.map(fetch,items))
    report = dict(status='sources-pinned',tables=tables,programs=rows,
        selection='Every distinct source on the official .NET measurement pages. No compressed-size selection.')
    (D / 'dotnet_sources.json').write_text(json.dumps(report,indent=2)+'\n')
    print('PINNED',len(rows),'sources', {l:sum(r['language']==l for r in rows) for l,_ in LANGS.values()},flush=True)

def sdk():
    meta = json.loads((D / 'support/dotnet-selected.json').read_text())
    asset = meta['asset']
    archive = D / 'support' / asset['url'].rsplit('/',1)[1]
    if not archive.exists():
        part = archive.with_suffix('.partial')
        with request(asset['url']) as src, part.open('wb') as dst:
            while block := src.read(1024*1024):
                dst.write(block)
        part.replace(archive)
    assert sha(archive,'sha512') == asset['hash']
    install = D / 'support/dotnet'
    install.mkdir(exist_ok=True)
    if not (install/'dotnet').exists():
        with tarfile.open(archive) as src:
            src.extractall(install,filter='data')
    meta.update(path=str(install/'dotnet'),sha256=sha(install/'dotnet'),
        archive=str(archive),archive_sha256=sha(archive),official_sha512_verified=True)
    (D / 'support/dotnet-sdk.json').write_text(json.dumps(meta,indent=2)+'\n')
    print('SDK INSTALLED',meta['version'],flush=True)

if __name__ == '__main__':
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        list(pool.map(lambda f:f(), [sdk,sources]))
