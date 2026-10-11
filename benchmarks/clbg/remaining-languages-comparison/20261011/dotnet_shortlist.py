"""Published elapsed times select candidates; they never become local measurements."""
import concurrent.futures
from html import unescape
import json
import re
from dotnet_fetch import D,sha
from dotnet_prepare import build,SDK
from metrics import normalize
from probe import save,WORK,validate_small

def hints(upstream):
    values={}
    text=(D/'upstream/dotnet'/(upstream+'-measurements.html')).read_text()
    for item in re.split(r'<tr\b[^>]*>',text)[1:]:
        ident=re.search(r'program/(\w+-'+upstream+r'-\d+)\.html',item)
        if not ident:continue
        cells=[unescape(re.sub(r'<[^>]*>','',c)).strip() for c in re.split(r'<td\b[^>]*>',item)[1:]]
        try:
            elapsed=float(re.match(r'[\d.,]+',cells[2])[0].replace(',',''))
            size=int(re.match(r'[\d,]+',cells[6])[0].replace(',',''))
        except (TypeError,IndexError,ValueError):continue
        name=ident[1]
        benchmark=name.split('-'+upstream+'-')[0]
        if size==WORK[benchmark][1]:values[name]=elapsed
    return values

def select():
    report=json.loads((D/'dotnet_jit_builds.json').read_text())
    small=json.loads((D/'results/dotnet-small.json').read_text())['programs']
    published=hints('csharpcore')|hints('fsharpcore')
    valid=[]
    for row in report['programs']:
        check=small.get(row['key'],{})
        if check.get('status')!='passed' or check.get('source_sha256')!=row['source_sha256']:continue
        row=dict(row,**normalize(open(row['source']).read(),row['language']))
        row['small_validation']=check['validation']
        row['published_elapsed_hint']=published.get(row['id'].split('-arm64')[0].removesuffix('-jit'))
        valid.append(row)
    selected=set()
    reasons={}
    def choose(row,reason):
        selected.add(row['id']);reasons.setdefault(row['id'],[]).append(reason)
    for lang in ['csharp','fsharp']:
        for bench in WORK:
            pool=[r for r in valid if r['language']==lang and r['benchmark']==bench]
            assert pool,(lang,bench)
            choose(min(pool,key=lambda r:(r['normalized_loc'],r['id'])),'Fewest normalized source lines among accepted candidates.')
            ranked=[r for r in pool if r['published_elapsed_hint'] is not None]
            if ranked:choose(min(ranked,key=lambda r:r['published_elapsed_hint']),'Lowest officially published elapsed time at the prescribed full workload; local timing still required.')
    preferred=['nbody-csharpcore-6','nbody-csharpcore-7','nbody-fsharpcore-5','nbody-fsharpcore-6',
        'fannkuchredux-csharpcore-5','revcomp-csharpcore-9','spectralnorm-csharpcore-5',
        'knucleotide-fsharp-local-rolling','regexredux-csharpcore-2']
    for row in valid:
        if any(row['id'].startswith(p+'-') for p in preferred):choose(row,'Additional ARM SIMD or complete rolling-map optimization candidate.')
    for row in valid:row['timing_selection_reasons']=reasons.get(row['id'],[])
    return [r for r in valid if r['id'] in selected],valid,report['excluded']

def main():
    selected,valid,excluded=select()
    print('SHORTLIST',len(selected),{l:sum(r['language']==l for r in selected) for l in ['csharp','fsharp']},flush=True)
    previous=D/'dotnet_aot_builds.json'
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        native=list(pool.map(lambda r:build(dict(r,id=r['id'].removesuffix('-jit')),True),selected))
    for row in native:
        if row['build_status']=='passed':
            try:row['small_validation']=validate_small(row)
            except Exception as e:row.update(preparation_status='failed',preparation_error=str(e))
    save(previous,dict(status='prepared',programs=native,excluded=[]))
    rows=selected+[r for r in native if r.get('small_validation',{}).get('reference_match') and not r.get('preparation_error')]
    report=dict(status='prepared',programs=rows,
        runtimes={'csharp':SDK,'fsharp':SDK,'csharp-aot':SDK,'fsharp-aot':SDK},
        libraries={p:sha(p)for p in ['/opt/homebrew/opt/gmp/lib/libgmp.10.dylib','/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib']},
        excluded=excluded+[r for r in native if r['build_status']!='passed' or r.get('preparation_error')],
        complete_accepted_pool=valid,
        selection='Measure every compact and published-fast endpoint plus native ARM optimization candidates. Preserve all original sources and compiled alternatives; do not display unmeasured alternatives.')
    save(D/'dotnet_manifest.json',report)
    print('PREPARED',len(rows),'executions',flush=True)

if __name__=='__main__':main()
