#!/usr/bin/env python3
"""Pin the listed k-nucleotide submissions without refreshing existing pins."""
import concurrent.futures
import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'nbody'))
from fetch_sources import Preformatted, download

HERE = Path(__file__).resolve().parent
BASE = 'https://benchmarksgame-team.pages.debian.net/benchmarksgame/'
EXT = {'chapel':'chpl','julia':'jl','rust':'rs','go':'go','graalvmaot':'java','gcc':'c','gpp':'cpp'}


def fetch(item):
    language, variant, secs, size, starred = item
    identifier = f'knucleotide-{language}-{variant}'
    url = BASE + 'program/' + identifier + '.html'
    parser = Preformatted()
    parser.feed(download(url))
    source = parser.blocks[0].strip() + '\n'
    path = f'upstream/{identifier}.{EXT[language]}'
    (HERE / path).write_text(source)
    return {'id':identifier,'language':{'gcc':'c','gpp':'cpp','graalvmaot':'java'}.get(language,language),
            'variant':variant,'path':path,'url':url,'sha256':hashlib.sha256(source.encode()).hexdigest(),
            'upstream_starred':starred,'published_secs':secs,'published_gz':size,
            'upstream_notes':parser.blocks[1].strip()}


def main():
    if (HERE/'manifest.json').exists():
        raise SystemExit('Manifest already pinned; remove it explicitly to refresh source pins.')
    (HERE/'upstream').mkdir(exist_ok=True)
    table = download(BASE+'performance/knucleotide.html')
    items=[]
    for row in re.split(r'<tr\b[^>]*>',table)[1:]:
        match=re.search(r'program/knucleotide-(chapel|julia|rust|go|graalvmaot|gcc|gpp)-(\d+)\.html',row)
        if not match: continue
        language,variant=match.groups()
        cells=[re.sub(r'<[^>]*>','',cell).strip() for cell in re.split(r'<td\b[^>]*>',row)[1:]]
        items.append((language,int(variant),float(cells[2]),int(cells[4].replace(',','')),'*' in row))
    if not items: raise RuntimeError('No source links found')
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        programs=list(pool.map(fetch,items))
    (HERE/'upstream/LICENSE.txt').write_bytes((HERE.parent/'nbody/upstream/LICENSE.txt').read_bytes())
    (HERE/'manifest.json').write_text(json.dumps({'schema_version':1,'retrieved_at':datetime.now(timezone.utc).isoformat(),
        'table_url':BASE+'performance/knucleotide.html',
        'selection':'All listed k-nucleotide submissions for Chapel, Julia, Rust, Go, Java native AOT, C and C++; starred entries retained. Architecture and dependency exclusions are explicit. Local Java is OpenJDK JIT.',
        'programs':programs},indent=2)+'\n')
    print(f'Pinned {len(programs)} complete sources with author comments, build notes and hashes.')


if __name__=='__main__':main()
