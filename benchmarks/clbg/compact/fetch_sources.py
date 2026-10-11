#!/usr/bin/env python3
"""Pin CLBG tables and all sources needed to audit compact-program selection."""
import argparse, concurrent.futures, gzip, hashlib, html, importlib.util, json, re, sys
from datetime import datetime, timezone
from pathlib import Path
from pygments import lex
from pygments.lexers import get_lexer_by_name
from pygments.token import Comment, Error

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
BASE = 'https://benchmarksgame-team.pages.debian.net/benchmarksgame/'
EXT = {'python3':'py','ruby':'rb','perl':'pl','lua':'lua','node':'js','julia':'jl','php':'php'}
LANG = {'python3':'python','node':'javascript'}
spec = importlib.util.spec_from_file_location('fetch_common', CLBG/'nbody/fetch_sources.py')
common = importlib.util.module_from_spec(spec); spec.loader.exec_module(common)
spec = importlib.util.spec_from_file_location('metrics_common', CLBG/'nbody/run.py')
metrics_common = importlib.util.module_from_spec(spec); spec.loader.exec_module(metrics_common)

def source_metrics(source, language):
    if language == 'julia':
        clean = metrics_common.strip_comments(source, language)
    else:
        tokens = list(lex(source, get_lexer_by_name(language, stripnl=False, ensurenl=False)))
        clean = ''.join(('\n'*v.count('\n') if t in Comment and t not in Comment.Preproc else v) for t,v in tokens)
    normalized = re.sub(r'\s+', ' ', clean).strip().encode()
    return dict(physical_lines=len(source.splitlines()),
                nonblank_noncomment_lines=sum(bool(s.strip()) for s in clean.splitlines()),
                normalized_bytes=len(normalized), gzip1_bytes=len(gzip.compress(normalized, compresslevel=1, mtime=0)),
                lexer_unclassified_tokens=[] if language=='julia' else [v for t,v in tokens if t in Error])

def fetch(row):
    parser = common.Preformatted(); parser.feed(common.download(row['url']))
    source = parser.blocks[0].strip()+'\n'
    path = 'upstream/'+row['id']+'.'+EXT[row['upstream_language']]
    (HERE/path).write_text(source)
    row.update(path=path,sha256=hashlib.sha256(source.encode()).hexdigest(),
               upstream_notes=parser.blocks[1].strip(), source=source_metrics(source,row['language']))
    return row

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--extend-php',action='store_true');ap.add_argument('--recount',action='store_true');args=ap.parse_args()
    existing=json.loads((HERE/'manifest.json').read_text()) if (HERE/'manifest.json').exists() else None
    if existing and not (args.extend_php or args.recount): raise SystemExit('Pins already exist; refusing refresh')
    if existing and args.extend_php and any(r['language']=='php' for r in existing['programs']):raise SystemExit('PHP pins already exist')
    for name in ('upstream','tables','results'): (HERE/name).mkdir(exist_ok=True)
    studies=json.loads((CLBG/'overview/normalized-results.json').read_text())
    all_rows=[]
    for study in studies:
        bench=study['key']; raw=(HERE/'tables'/f'{bench}.html').read_text() if existing else common.download(BASE+'performance/'+bench+'.html')
        if not existing:(HERE/'tables'/f'{bench}.html').write_text(raw)
        for row in re.split(r'<tr\b[^>]*>',raw)[1:]:
            m=re.search(r'program/('+bench+r'-(\w+)(?:-(\d+))?)\.html',row)
            if not m: continue
            identifier,lang,variant=m.groups(); variant=int(variant or 1)
            cells=[html.unescape(re.sub(r'<[^>]*>','',s)).strip() for s in re.split(r'<td\b[^>]*>',row)[1:]]
            if len(cells)<5 or not re.fullmatch(r'[0-9,]+',cells[4]): continue
            all_rows.append(dict(id=identifier,benchmark=bench,upstream_language=lang,
                 language=LANG.get(lang,lang),variant=variant,url=BASE+m.group(0),
                 published_gz=int(cells[4].replace(',','')),published_time=cells[2],upstream_starred='*' in cells[1]))
    wanted=[] if args.recount else [r for r in all_rows if r['upstream_language'] in EXT and (not existing or r['upstream_language']=='php')]
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        pins=list(pool.map(fetch,wanted))
    if existing:pins=existing['programs']+pins
    for row in pins:
        row.pop('selection_reasons',None)
        if args.recount:row['source']=source_metrics((HERE/row['path']).read_text(),row['language'])
    selected=[]
    audit=[]
    for study in studies:
        bench=study['key']; compact=min((r for r in study['rows'] if r['lang']=='mech'),key=lambda r:r['gzip'])
        for lang in EXT:
            rows=[r for r in pins if r['benchmark']==bench and r['upstream_language']==lang]
            if not rows: continue
            pub=min(r['published_gz'] for r in rows)
            local=min(r['source']['gzip1_bytes'] for r in rows)
            loc=min(r['source']['nonblank_noncomment_lines'] for r in rows)
            ids=[]
            for r in rows:
                reasons=[]
                if r['published_gz']==pub: reasons.append('minimum listed CLBG gz (ties retained)')
                if r['source']['gzip1_bytes']==local: reasons.append('minimum local normalized gzip (ties retained)')
                if r['source']['nonblank_noncomment_lines']==loc: reasons.append('minimum noncomment physical LOC (ties retained)')
                if reasons:
                    r['selection_reasons']=reasons; selected.append(r['id']); ids.append(r['id'])
            audit.append(dict(benchmark=bench,language=LANG.get(lang,lang),listed_candidates=len(rows),
                     smallest_published_gz=pub,smallest_local_gzip=local,smallest_loc=loc,
                     mech_gzip=compact['gzip'],mech_loc=min(r['loc'] for r in study['rows'] if r['lang']=='mech'),selected_ids=ids))
    manifest=dict(schema_version=1,retrieved_at=datetime.now(timezone.utc).isoformat(),
        selection='Audit every currently listed source for Python 3, Ruby, Perl, Lua, Node.js, PHP and Julia. Measure the minimum listed gz, local normalized gzip, and noncomment physical LOC; retain all ties. Compact-only cohort: no claim about fastest entry in these newly added languages.',
        source_metric='Language-aware comment removal (Pygments 2.19.2 for added languages; existing Julia scanner), whitespace collapse, UTF-8, gzip level 1, mtime=0. Local metric, not byte-identical to CLBG gz. Complete source; no reformatting.',
        tables=[dict(path='tables/'+s['key']+'.html',url=BASE+'performance/'+s['key']+'.html',sha256=hashlib.sha256((HERE/'tables'/(s['key']+'.html')).read_bytes()).hexdigest()) for s in studies],
        programs=pins,selected_ids=selected,audit=audit,all_listed=all_rows)
    (HERE/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    (HERE/'upstream/LICENSE.txt').write_bytes((CLBG/'nbody/upstream/LICENSE.txt').read_bytes())
    for a in audit: print(a['benchmark'],a['language'],a['listed_candidates'],a['smallest_local_gzip'],a['smallest_loc'],','.join(a['selected_ids']))
    print('Pinned',len(pins),'sources;',len(selected),'selected entries (including Julia audit)')

if __name__=='__main__': main()
