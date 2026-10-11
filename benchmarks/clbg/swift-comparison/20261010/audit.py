"""Pin the source/work review and unchanged current Mech control executables."""
import json,re
from pathlib import Path
import run as runner
D=Path(__file__).resolve().parent;C=D.parents[1]
sha=runner.sha
SITE=Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site/dist/data.json')
def main():
    built=json.loads((D/'results/builds.json').read_text())
    data=json.loads(SITE.read_text());programs=data['programs']
    programs=list(programs.values()) if isinstance(programs,dict) else programs
    fixed=json.loads((C/'work-audit/20261009/builds.json').read_text())['programs']
    controls=[]
    for bench in runner.BENCHES:
        published=min((r for r in programs if r['benchmark']==bench and r['language']=='mech'),key=lambda r:r['seconds'])
        report=json.loads((C/published['report']).read_text());records=report['programs']
        records=list(records.values()) if isinstance(records,dict) else records
        matches=[r for r in records if r.get('binary_sha256')==published['binary_sha256']]
        assert len(matches)==1,(bench,len(matches))
        row=matches[0].copy()
        if 'source' not in row:
            correction=next(r for r in fixed.values() if r['binary_sha256']==row['binary_sha256'])
            row.update({k:correction[k] for k in ('source','source_sha256','binary')})
        binary=row.get('binary') or row['command'][0]
        assert sha(binary)==published['binary_sha256'] and sha(row['source'])==row['source_sha256']
        controls.append(dict(id='mech-control-'+bench,key=bench+':mech-control',benchmark=bench,language='mech',
            source=row['source'],source_sha256=row['source_sha256'],source_url='https://mech-lang.org',binary=binary,
            binary_sha256=sha(binary),command=row['command'],runtime_environment=row.get('runtime_environment',{}),
            loc=published['loc'],published_chart_key=published['key'],previous_seconds=published['seconds'],
            previous_report=published['report'],algorithm=published['algorithm']))
    rows=[]
    for row in built['programs']:
        reason=runner.exclusion(row)
        rows.append(dict(key=row['key'],id=row['id'],benchmark=row['benchmark'],source=row['source'],
            sha256=row['source_sha256'],loc=row['source_metrics']['nonblank_noncomment_lines'],admitted=reason is None,
            reason=reason or runner.REASONS[row['benchmark']],source_url=row['source_url'],
            local_adaptation=row.get('local_adaptation',False)))
    ir={str(p.relative_to(D)):sha(p) for p in sorted((D/'results').glob('*.ll'))}
    for n in (1,2,3,4,5):
        text=(D/'results'/f'binarytrees-swift-{n}-faithful.ll').read_text()
        funcs=re.findall(r'^define [^\n]+\{\n.*?^}',text,re.M|re.S)
        constructor=next(f for f in funcs if any(x in f.splitlines()[0] for x in ('bottomUpTree','createTree')))
        assert constructor.count('@swift_allocObject')==2,(n,'interior/leaf allocation')
        assert constructor.count('bottomUpTree' if n!=1 else 'createTree')==3,(n,'two recursive child constructions')
        main_ir=next(f for f in funcs if 'main(' in f.splitlines()[0])
        wait=main_ir.index('4waityyF')
        assert any(x in main_ir[wait:] for x in ('itemCheck','5check')),(n,'final check after waiting')
    audit=dict(status='reviewed',builds_sha256=sha(D/'results/builds.json'),rows=rows,controls=controls,ir_pins=ir,
        tree_review=dict(status='reviewed',compiler=built['compiler']['version'],findings={
            'enum_leaves':'Both recursive and depth-zero branches call swift_allocObject for a 32-byte box containing two child slots. Leaf allocation is present; no shared Empty leaf replaces Node(Empty,Empty).',
            'temporary_work':'The optimized enum inner loop calls bottomUpTree, then itemCheck, then swift_release on every iteration; recursive constructors allocate every node. Class constructors likewise allocate both interior and leaf nodes.',
            'original_lifetime_2_3_4':'The long-lived tree is owned by a queued check closure and can be released when that closure completes, without waiting for every temporary-tree task. No long-lived root is used after group.wait.',
            'original_stretch_1_5':'The stretch tree is stored in a global strong root and is not cleared before long-lived allocation.',
            'corrected_lifetime':'Each corrected source completes and releases stretch storage before constructing a global long-lived root. The final check occurs after group.wait. Optimized IR preserves the allocation, release and check ordering.'}),
        policy={
            'output':'Exact complete byte hashes at small and official full workloads, with nbody energy tolerance 1e-8. No admission solely for correct printed output.',
            'loc':'Nonblank/noncomment source lines, with packed independent semicolon-separated statements separated. All Swift algorithm code and lifecycle corrections count. Original names retained; no compression metric.',
            'scheduling':'Apple M3, eight logical CPUs; original Swift Dispatch scheduling retained. Task counts may exceed core counts, as task scheduling differs from available CPU parallelism.',
            'upstream':'All 37 Swift sources linked by the pinned published measurement page were considered. Failed builds, work mismatches and later output failures remain explicitly recorded. Corrections retain original sources and provenance.'},
        requirements={b:'https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/'+b+'.html' for b in runner.BENCHES})
    (D/'source-audit.json').write_text(json.dumps(audit,indent=2)+'\n')
    print('Admitted Swift candidates',sum(r['admitted'] for r in rows),'Mech controls',len(controls),'IR files',len(ir))
    print('Candidates per benchmark',{b:sum(r['admitted'] and r['benchmark']==b for r in rows) for b in runner.BENCHES})
if __name__=='__main__':main()
