#!/usr/bin/env python3
"""Export white-background LOC corner charts from the audited AC rerun."""
import argparse
import hashlib
import json
import math
from pathlib import Path
from language_colors import COLORS, LINE_COLORS, OUTLINE_COLORS

HERE=Path(__file__).resolve().parent
CLBG=HERE.parent
LABELS=dict(c='C (GCC + LLVM)',chapel='Chapel',cpp='C++ (GCC + LLVM)',go='Go',
            java='Java',julia='Julia',mech='Mech (AOT)',rust='Rust',
            python='Python (CPython)',ruby='Ruby (YJIT)',perl='Perl',lua='Lua',
            luajit='LuaJIT',javascript='JavaScript (V8)',php='PHP (JIT)')
NATIVE=list(LABELS)[:8]


def gm(values):return math.exp(sum(math.log(v) for v in values)/len(values))


def models():
    studies=json.loads((HERE/'normalized-results.json').read_text())
    audit=json.loads((CLBG/'retime/results/audit.json').read_text())
    report=CLBG/'retime/results'/audit['report']
    assert audit['passed'] and hashlib.sha256(report.read_bytes()).hexdigest()==audit['sha256']
    assert len(studies)==8 and all(s['sha256']==audit['sha256'] for s in studies)
    output=[]
    for study in studies:
        rows=study['rows'];bs=min(r['loc'] for r in rows);bt=min(r['seconds'] for r in rows)
        groups=[]
        for lang,label in LABELS.items():
            candidates=[r for r in rows if r['lang']==lang];assert candidates
            compact=all(r.get('endpoint_scope')=='compact-only' for r in candidates)
            def point(row):
                return dict(id=row['id'],x=row['loc']/bs,y=row['seconds']/bt,
                    seconds=row['seconds'],loc=row['loc'],
                    algorithm=row.get('algorithm'),modified=row.get('modified',False))
            groups.append(dict(lang=lang,label=label,compact_only=compact,count=len(candidates),
                fast=None if compact else point(min(candidates,key=lambda r:(r['seconds'],r['loc'],r['id']))),
                small=point(min(candidates,key=lambda r:(r['loc'],r['seconds'],r['id']))),
                builds=[point(r) for r in candidates]))
        output.append(dict(key=study['key'],title=study['name'],workload=study['workload'],size_baseline_loc=bs,groups=groups))
    summary=[]
    for lang,label in LABELS.items():
        groups=[next(g for g in s['groups'] if g['lang']==lang) for s in output]
        def mean_point(kind):
            points=[g[kind] for g in groups]
            return None if any(p is None for p in points) else dict(x=gm([p['x'] for p in points]),y=gm([p['y'] for p in points]))
        summary.append(dict(lang=lang,label=label,compact_only=groups[0]['compact_only'],
            fast=mean_point('fast'),small=mean_point('small'),builds=[],count=sum(g['count'] for g in groups)))
    return dict(source_metric='nonblank_noncomment_lines',summary=dict(key='summary',title='All eight benchmarks',workload='Equal-weight geometric means',groups=summary),studies=output,audit_sha256=audit['sha256'])


def export_figures(data):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    from matplotlib.ticker import FuncFormatter,LogLocator
    colors=COLORS
    for model in [data['summary'],*data['studies']]:
        for zoom in ([False,True] if model['key']=='summary' else [False]):
            groups=[g for g in model['groups'] if not zoom or g['lang'] in NATIVE]
            points=[p for g in groups for p in [*g['builds'],g['fast'],g['small']] if p]
            combined=model['key']=='summary'
            xfactor=1 if combined else model['size_baseline_loc']
            x=lambda p:p['x']*xfactor
            fig,ax=plt.subplots(figsize=(13,9),facecolor='white')
            ax.set_facecolor('white')
            fig.subplots_adjust(left=.09,right=.985,top=.88,bottom=.19)
            ax.set_xscale('log',base=2);ax.set_yscale('log',base=2)
            ax.set_xlim(min(1,min(p['x'] for p in points))*xfactor*.94,max(x(p) for p in points)*1.13)
            ax.set_ylim(min(1,min(p['y'] for p in points))*.87,max(p['y'] for p in points)*1.22)
            for axis in (ax.xaxis,ax.yaxis):
                axis.set_major_locator(LogLocator(base=2,numticks=12))
                axis.set_major_formatter(FuncFormatter(lambda v,_:f'{v:g}×'))
            if not combined:ax.xaxis.set_major_formatter(FuncFormatter(lambda v,_:f'{v:g}'))
            ax.grid(color='#dfe3e8',linewidth=.7);ax.set_axisbelow(True)
            for group in groups:
                color=colors[group['lang']];fast,small=group['fast'],group['small']
                for p in group['builds']:ax.scatter(x(p),p['y'],s=16,color=color,alpha=.20,zorder=2)
                same=fast and math.isclose(fast['x'],small['x']) and math.isclose(fast['y'],small['y'])
                if fast and not same:ax.plot([x(small),x(fast)],[small['y'],fast['y']],color=LINE_COLORS[group['lang']],linewidth=1.8)
                label=group['label']+(' · smallest only' if group['compact_only'] else '')+f' ({group["count"]} builds)'
                if fast:ax.scatter(x(fast),fast['y'],s=64,marker='o',color=color,edgecolor=OUTLINE_COLORS[group['lang']],linewidth=.7,label=label,zorder=5)
                if not same:ax.scatter(x(small),small['y'],s=60,marker='s',color=color,edgecolor=OUTLINE_COLORS[group['lang']],linewidth=.7,label=label if not fast else None,zorder=4)
            ax.legend(loc='upper right',fontsize=8.4,framealpha=.94)
            mech=next(g for g in groups if g['lang']=='mech')['fast']
            annotation=(f'Mech: {mech["loc"]} LOC · {mech["seconds"]:.3f}s' if 'seconds' in mech
                else f'Mech: {mech["x"]:.2f}× LOC · {mech["y"]:.2f}× time')
            ax.annotate(annotation,(x(mech),mech['y']),xytext=(12,12),textcoords='offset points',fontsize=10,color=LINE_COLORS['mech'])
            title=model['title']+': execution time and lines of code'
            subtitle=('Eight benchmarks · geometric means'+(' · native-language corner' if zoom else ' · full cohort') if model['key']=='summary'
                else 'Apple M3; '+model['workload']+'; median of 3 fresh processes')
            ax.set_title(title+'\n'+subtitle,loc='left',fontsize=17,pad=16)
            ax.set_xlabel('Source lines / fewest source lines — fewer ←' if combined else 'Source lines of code (LOC) — fewer ←',fontsize=12)
            ax.set_ylabel('Normalized elapsed time — faster ↓',fontsize=12)
            fig.text(.05,.08,'Circle: fastest measured; square: fewest LOC; faint points: every measured build; line joins a language’s endpoints.\n'
                'Apple M3 · AC power · Low Power Mode off · startup and full output included · complete Mech source counted.\n'
                'Python/Ruby/Perl/PHP/JavaScript/Lua/LuaJIT: smallest listings only; their fastest programs were not sampled.\n'
                'LOC excludes blank and comment-only lines. Generic Mech runtime excluded. Algorithm/allocation differences retained.',fontsize=9,color='#505863')
            stem=HERE/f'original-style-{model["key"]}{"-native" if zoom else ""}'
            for ext in ('png','svg'):fig.savefig(str(stem)+'.'+ext,dpi=160)
            plt.close(fig)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--no-exports',action='store_true');args=parser.parse_args()
    data=models();(HERE/'original-style-results.json').write_text(json.dumps(data,indent=2)+'\n')
    if not args.no_exports:export_figures(data)
    print('Updated white-background LOC charts: eight benchmarks, 575 builds, 15 runtimes; audited AC measurements')


if __name__=='__main__':main()
