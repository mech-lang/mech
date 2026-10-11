#!/usr/bin/env python3
"""Pin complete listed sources for the final two CLBG benchmarks."""
import argparse, concurrent.futures, hashlib, json, re, sys
from datetime import datetime, timezone
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent / 'nbody'))
from fetch_sources import Preformatted, download

BASE = 'https://benchmarksgame-team.pages.debian.net/benchmarksgame/'
EXT = {'chapel':'chpl','julia':'jl','rust':'rs','go':'go','graalvmaot':'java',
       'java':'java','gcc':'c','gpp':'cpp','python3':'py','ruby':'rb','perl':'pl',
       'lua':'lua','luajit':'lua','node':'js','php':'php','racket':'rkt','tcl':'tcl'}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('benchmark', choices=['regexredux','pidigits'])
    args = parser.parse_args()
    here = Path(__file__).resolve().parent / args.benchmark
    if (here/'manifest.json').exists():
        raise SystemExit('Sources already pinned; refusing to refresh them.')
    (here/'upstream').mkdir(parents=True, exist_ok=True)
    table_url = BASE + 'performance/' + args.benchmark + '.html'
    table = download(table_url)
    items = []
    for row in re.split(r'<tr\b[^>]*>', table)[1:]:
        match = re.search(r'program/('+args.benchmark+r'-([a-z0-9]+)(?:-(\d+))?)\.html', row)
        if not match or match[2] not in EXT: continue
        identifier, lang, variant = match.groups()
        cells = [re.sub(r'<[^>]*>','', c).strip() for c in re.split(r'<td\b[^>]*>',row)[1:]]
        items.append((identifier, lang, int(variant or 1),
                      float(cells[2]) if re.fullmatch(r'[0-9.]+',cells[2]) else None,
                      '*' in row))
    def fetch(item):
        identifier, lang, variant, seconds, starred = item
        url = BASE+'program/'+identifier+'.html'
        page = Preformatted(); page.feed(download(url))
        source = page.blocks[0].strip()+'\n'
        path = 'upstream/'+identifier+'.'+EXT[lang]
        (here/path).write_text(source)
        return dict(id=identifier, language={'gcc':'c','gpp':'cpp','graalvmaot':'java',
          'python3':'python','node':'javascript'}.get(lang,lang), variant=variant,
          path=path,url=url,sha256=hashlib.sha256(source.encode()).hexdigest(),
          upstream_starred=starred,published_secs=seconds,upstream_notes=page.blocks[1].strip())
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        programs = list(pool.map(fetch, items))
    if not programs: raise RuntimeError('No listed sources found')
    (here/'upstream/LICENSE.txt').write_bytes((here.parent/'nbody/upstream/LICENSE.txt').read_bytes())
    (here/'manifest.json').write_text(json.dumps(dict(schema_version=1,
      retrieved_at=datetime.now(timezone.utc).isoformat(),table_url=table_url,
      selection='All listed sources in the configured native and compact language cohort; source/build/architecture exclusions recorded during measurement.',
      programs=programs),indent=2)+'\n')
    print(args.benchmark, 'pinned',len(programs),'complete sources')

if __name__ == '__main__': main()
