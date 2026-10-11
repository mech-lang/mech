"""Pin work/LOC review, optimized allocation evidence, and unchanged Mech controls."""
import json,re
from pathlib import Path
import run as runner
D=Path(__file__).resolve().parent;C=D.parents[1];sha=runner.sha
SITE=Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site/dist/data.json')
def main():
    built=json.loads((D/'results/builds.json').read_text());data=json.loads(SITE.read_text());programs=list(data['programs'].values())
    fixed=json.loads((C/'work-audit/20261009/builds.json').read_text())['programs'];controls=[]
    for bench in runner.BENCHES:
        published=min((r for r in programs if r['benchmark']==bench and r['language']=='mech'),key=lambda r:r['seconds'])
        report=json.loads((C/published['report']).read_text());records=report['programs'];records=list(records.values()) if isinstance(records,dict) else records
        row=next(r.copy() for r in records if r.get('binary_sha256')==published['binary_sha256'])
        if 'source' not in row:
            correction=next(r for r in fixed.values() if r['binary_sha256']==row['binary_sha256']);row.update({k:correction[k] for k in ('source','source_sha256','binary')})
        binary=row.get('binary') or row['command'][0];assert sha(binary)==published['binary_sha256'] and sha(row['source'])==row['source_sha256']
        controls.append(dict(id='mech-control-'+bench,key=bench+':mech-control',benchmark=bench,language='mech',source=row['source'],source_sha256=row['source_sha256'],source_url='https://mech-lang.org',binary=binary,binary_sha256=sha(binary),command=row['command'],runtime_environment=row.get('runtime_environment',{}),loc=published['loc'],published_chart_key=published['key'],previous_seconds=published['seconds'],previous_report=published['report'],algorithm=published['algorithm']))
    rows=[]
    for row in built['programs']:
        reason=runner.exclusion(row);a=row['loc_audit']
        rows.append(dict(key=row['key'],id=row['id'],benchmark=row['benchmark'],language=row['language'],source=row['source'],sha256=row['source_sha256'],loc=a['normalized_loc'],physical_loc=a['physical_loc'],expansions=a['expansions'],admitted=reason is None,reason=reason or runner.REASONS[row['benchmark']],source_url=row['source_url'],local_adaptation=row.get('local_adaptation',False)))
    evidence=[]
    for row in built['programs']:
        if row['benchmark']!='binarytrees' or not row.get('local_adaptation') or row['build_status']!='passed':continue
        if row['language']=='haskell':
            path=Path(row['source']).with_suffix('.thr.dump-stg-final');text=path.read_text()
            assert 'leafNil' in text and text.count('Main.Node [')>=2,row['id']
            assert re.search(r'case (?:\$\w+make\w*|make\w*)',text),row['id']
            note='Optimized final STG preserves two recursively constructed children and dynamic Node allocation in both leaf and recursive constructor branches. sumT calls construction and traversal inside the counter loop. Strict fields ensure a complete tree before walking. The long-lived root is captured in the continuation used after temporary work.'
        else:
            path=Path(row['source']).with_suffix('.s');text=path.read_text()
            assert len(re.findall(r'sub\s+x27,\s*x27,\s*#24',text))>=2,row['id']
            assert len(re.findall(r'bl\s+_caml.*\$make_',text))>=2,row['id']
            note='Optimized AArch64 assembly advances the allocation pointer by 24 bytes in both depth-zero and recursive paths. Two recursive calls construct independent children. A global GC root retains the long-lived tree until the final traversal; the temporary loop calls construction/traversal on every iteration.'
        evidence.append(dict(id=row['id'],file=str(path.relative_to(D)),sha256=sha(path),finding=note))
    assert len(evidence)==9,len(evidence)
    report=dict(status='reviewed',builds_sha256=sha(D/'results/builds.json'),rows=rows,controls=controls,allocation_evidence=evidence,
        policy=dict(output='Small and full complete output hashes; nbody energy tolerance 1e-8. Source/compiler work review is required beyond matching printed answers.',loc='Nonblank/noncomment physical LOC plus packed independent semicolon statements and multiple independent bindings. Structural list/record/tuple separators are excluded. Haskell semantic pragmas count. All work corrections and API ports count; general libraries/compiler excluded.',
        scheduling='Eight logical CPUs; eight Haskell RTS capabilities. Published OCaml fork/task partitioning retained. Default GC and no heap-tuning flags for trees.',upstream='All 60 source candidates from the two pinned published measurements pages were considered, with original sources and separately labelled corrections retained locally. Only locally measured eligible endpoints enter the website.'),
        requirements={b:'https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/'+b+'.html' for b in runner.BENCHES})
    (D/'source-audit.json').write_text(json.dumps(report,indent=2)+'\n');print('Admitted',sum(r['admitted'] for r in rows),'sources; controls',len(controls),'allocation evidence',len(evidence))
if __name__=='__main__':main()
