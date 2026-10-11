#!/usr/bin/env python3
"""Extend the preserved white LOC charts with confirmed nbody and final tasks."""
import hashlib, json, math
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Patch
from build_original_style import LABELS, gm, models
from plot_mega import canvas, footer, limits, save, same_point, spread, annotate_without_overlap
from language_colors import COLORS, LINE_COLORS, OUTLINE_COLORS, MECH_PROFILES

HERE=Path(__file__).resolve().parent;CLBG=HERE.parent
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def point(row):
    return dict(id=row['id'],lang=row['language'],seconds=row['median_seconds'],
                loc=row.get('loc',row.get('source',{}).get('nonblank_noncomment_lines') if isinstance(row.get('source'),dict) else None),
                algorithm=row.get('algorithm'),runtime_policy=row.get('runtime_policy'))
def group_rows(key,title,rows,compact_languages):
    size=min(p['loc'] for p in rows);time=min(p['seconds'] for p in rows);groups=[]
    for lang,label in LABELS.items():
        if lang=='php':label='PHP'
        candidates=[dict(p,x=p['loc']/size,y=p['seconds']/time) for p in rows if p['lang']==lang]
        if not candidates:continue
        groups.append(dict(lang=lang,label=label,compact_only=lang in compact_languages,count=len(candidates),builds=candidates,
          small=min(candidates,key=lambda p:(p['loc'],p['seconds'])),
          fast=None if lang in compact_languages else min(candidates,key=lambda p:(p['seconds'],p['loc']))))
    return dict(key=key,title=title,groups=groups,size_baseline_loc=size,time_baseline_seconds=time)

def data_model(confirmation_path=None):
    previous=models();studies=[];provenance={}
    native=set(list(LABELS)[:8]);compact=set(LABELS)-native
    scripting_path=CLBG/'fast-scripting/results/apple-m3-ac-normal-20261009.json'
    scripting=json.loads(scripting_path.read_text()) if scripting_path.exists() else None
    if scripting and scripting['status']=='complete':
        provenance[str(scripting_path.relative_to(CLBG))]=sha(scripting_path)
        compact=set()
    else:
        scripting=None
    policies_path=CLBG/'fast-scripting/results/runtime-policies.json'
    policies=json.loads(policies_path.read_text()) if scripting and policies_path.exists() else None
    if policies and policies['status']=='complete':
        assert policies['primary_report_sha256']==sha(scripting_path)
        provenance[str(policies_path.relative_to(CLBG))]=sha(policies_path)
    else:policies=None
    npath=CLBG/'nbody/experiments/20261009-local-simd/final-confirmation-50m.json'
    nbody=json.loads(npath.read_text());provenance[str(npath.relative_to(CLBG))]=sha(npath)
    for study in previous['studies']:
        rows=[dict(p,lang=g['lang']) for g in study['groups'] for p in g['builds']]
        if scripting:
            for new in [*scripting['programs'],*(policies['programs'] if policies else [])]:
                if new['benchmark']!=study['key'] or new.get('excluded_full') or 'median_seconds' not in new:continue
                assert len(new['samples_seconds'])>=3 and all(v['reference_match'] for v in new['sample_validation'])
                rows=[p for p in rows if p['id']!=new['id']];rows.append(point(new))
        if study['key']=='nbody':
            rows=[p for p in rows if p['lang']!='mech']
            for new in nbody['programs']:
                if new['id'].startswith('before-'):continue
                rows=[p for p in rows if p['id']!=new['id']];rows.append(point(new))
        title={'nbody':'N-body','knucleotide':'K-nucleotide'}.get(study['key'],study['title'])
        studies.append(group_rows(study['key'],title,rows,compact))
    final_path=confirmation_path or CLBG/'remaining-conformance/signed-division-confirmation.json'
    final=json.loads(final_path.read_text());assert all(len(p['samples_seconds'])==7 for p in final['programs'])
    provenance[str(final_path.relative_to(CLBG))]=sha(final_path)
    for key,title in [('regexredux','Regex-redux'),('pidigits','Pi digits')]:
        path=CLBG/key/'results/apple-m3-ac-normal-20261009.json';report=json.loads(path.read_text());provenance[str(path.relative_to(CLBG))]=sha(path)
        rows=[point(p) for p in report['programs'] if p['language']!='mech']
        for row in final['programs']:
            if row['benchmark']!=key:continue
            rows=[p for p in rows if p['id']!=row['id']];rows.append(point(row))
        studies.append(group_rows(key,title,rows,compact))
    summary=[];missing=[]
    for lang,label in LABELS.items():
        if lang=='php':label='PHP'
        inputs=[g for s in studies for g in s['groups'] if g['lang']==lang]
        if len(inputs)!=10:
            missing.append(dict(lang=lang,covered=len(inputs),missing=[s['key'] for s in studies if not any(g['lang']==lang for g in s['groups'])]));continue
        def mean(kind):
            points=[g[kind] for g in inputs]
            return None if any(p is None for p in points) else dict(x=gm([p['x'] for p in points]),y=gm([p['y'] for p in points]))
        summary.append(dict(lang=lang,label=label,compact_only=lang in compact,fast=mean('fast'),small=mean('small')))
    assert len(studies)==10 and len(summary)==14
    return dict(studies=studies,summary=summary,missing=missing,provenance=provenance,
                scripting_fastest_measured=bool(scripting),
                scripting_excluded=scripting['excluded'] if scripting else [],
                previous_audit_sha256=previous['audit_sha256'])

def mega(data):
    fig,ax=canvas('Language summary across all ten benchmarks: LOC and speed',
                  'One geometric-mean line per language · faint dots show the benchmark inputs')
    inputs=[p for s in data['studies'] for g in s['groups'] if g['lang']!='lua' for p in (g['small'],g['fast']) if p]
    endpoints=[p for g in data['summary'] for p in (g['small'],g['fast']) if p]
    limits(ax,[*inputs,*endpoints,dict(x=1,y=1)])
    for study in data['studies']:
        for g in study['groups']:
            if g['lang']=='lua':continue
            for kind,marker in [('small','s'),('fast','o')]:
                p=g[kind]
                if p:ax.scatter(p['x'],p['y'],marker=marker,s=29,color=COLORS[g['lang']],edgecolor=OUTLINE_COLORS[g['lang']],linewidth=.35,alpha=.20,zorder=2)
    handles=[];lines=0
    for g in data['summary']:
        lang=g['lang'];fast,small=g['fast'],g['small'];color=COLORS[lang]
        if fast and not same_point(fast,small):
            ax.plot([small['x'],fast['x']],[small['y'],fast['y']],color=LINE_COLORS[lang],linewidth=2.4 if lang=='mech' else 1.8,zorder=4);lines+=1
        if fast:ax.scatter(fast['x'],fast['y'],marker='o',s=95 if lang=='mech' else 67,color=color,edgecolor=OUTLINE_COLORS[lang],linewidth=.7,zorder=6)
        if not same_point(fast,small):ax.scatter(small['x'],small['y'],marker='s',s=95 if lang=='mech' else 67,color=color,edgecolor=OUTLINE_COLORS[lang],linewidth=.7,zorder=6)
        handles.append(Line2D([],[],color=LINE_COLORS[lang],markerfacecolor=color,markeredgecolor=OUTLINE_COLORS[lang],markeredgewidth=.7,marker='s' if g['compact_only'] else 'o',linestyle='-',linewidth=1.8,label=g['label']+(' · fewest LOC only' if g['compact_only'] else '')))
    legend=ax.legend(handles=handles,loc='upper right',ncols=2,fontsize=10,framealpha=1,
                     borderpad=.8,handlelength=2,columnspacing=1.6,labelspacing=.6)
    legend.set_zorder(20)
    mech=next(g for g in data['summary'] if g['lang']=='mech')['fast']
    ax.annotate(f'Mech: {mech["x"]:.2f}× LOC · {mech["y"]:.2f}× time',(mech['x'],mech['y']),xytext=(12,12),textcoords='offset points',color=LINE_COLORS['mech'],fontsize=10)
    footer(fig,['Circle: fastest measured; square: fewest LOC. Bold endpoints are equal-weight geometric means over the same ten benchmarks.',
      'Faint matching markers show the individual benchmark endpoints used in each mean. Normalize per benchmark before aggregating.',
      'Apple M3 · AC power · Low Power Mode off · October 8–9 · 3 samples in the broad sweep; 7 in latest confirmations.',
      'Scripting cohort: fewest-LOC entries plus published-fastest candidates; fastest is the best measured valid program.' if data['scripting_fastest_measured'] else 'Compact runtimes retain fewest-LOC endpoints only because the first eight tasks did not sample their fastest variants.',
      'Complete source LOC; generic libraries excluded.'])
    save(fig,'mega-ten.loc');plt.close(fig)
    return dict(lines=lines,aggregated_languages=len(data['summary']),faint_points=len(inputs),omitted_languages=['lua'])

def mech_spread(data):
    groups=[(s,next(g for g in s['groups'] if g['lang']=='mech')) for s in data['studies']]
    regions={kind:spread([g[kind] for _,g in groups]) for kind in ('fast','small')}
    fig,ax=canvas('Mech: all ten benchmarks and standard deviation',
                  'AOT · geometric means and spread across benchmarks')
    points=[];specs=[];handles=[]
    for kind,color in [('small',MECH_PROFILES['small']),('fast',MECH_PROFILES['fast'])]:
        boundary=regions[kind]['boundary'];ax.fill([p['x'] for p in boundary],[p['y'] for p in boundary],color=color,alpha=.16,zorder=1)
        ax.plot([p['x'] for p in boundary],[p['y'] for p in boundary],color=LINE_COLORS['mech'],linestyle='--' if kind=='small' else '-',linewidth=1,alpha=.65,zorder=2)
        points.extend(boundary)
    merged={}
    for s,g in groups:
        fast,small=g['fast'],g['small'];points.extend([fast,small])
        if not same_point(fast,small):
            ax.plot([small['x'],fast['x']],[small['y'],fast['y']],color=LINE_COLORS['mech'],linewidth=1.6,zorder=4)
            ax.scatter(small['x'],small['y'],marker='s',s=67,color=COLORS['mech'],edgecolor=LINE_COLORS['mech'],linewidth=.7,zorder=5)
            merged.setdefault((round(small['x'],10),round(small['y'],10)),[small,[]])[1].append(f'{s["title"]} ({small["loc"]} LOC)')
        ax.scatter(fast['x'],fast['y'],marker='o',s=67,color=COLORS['mech'],edgecolor=LINE_COLORS['mech'],linewidth=.7,zorder=5)
        merged.setdefault((round(fast['x'],10),round(fast['y'],10)),[fast,[]])[1].append(f'{s["title"]} ({fast["loc"]} LOC)')
    for p,names in merged.values():specs.append(('\n'.join(names),p,(10,14)))
    for kind,marker,title in [('fast','o','Fastest'),('small','s','Fewest LOC')]:
        p=regions[kind]['center'];points.append(p);ax.scatter(p['x'],p['y'],marker=marker,s=155,color=COLORS['mech'],edgecolor='#222',zorder=8)
        handles.append(Line2D([],[],marker=marker,markerfacecolor=COLORS['mech'],markeredgecolor='#222',linestyle='None',label=f'{title} geometric mean: {p["x"]:.2f}× LOC · {p["y"]:.2f}× time'))
    handles.extend([Patch(facecolor=MECH_PROFILES['fast'],alpha=.16,label='Fastest profile: one standard deviation'),Patch(facecolor=MECH_PROFILES['small'],alpha=.16,label='Fewest-LOC profile: one standard deviation')])
    limits(ax,points);ax.set_xlim(min(p['x'] for p in points)*.8,max(p['x'] for p in points)*1.4);ax.set_ylim(min(p['y'] for p in points)*.8,max(p['y'] for p in points)*1.3)
    legend=ax.legend(handles=handles,loc='upper right',fontsize=9,framealpha=.96)
    annotate_without_overlap(fig,ax,specs,[g[k] for _,g in groups for k in ('fast','small')],legend)
    footer(fig,['Circle: fastest measured; square: fewest LOC. Large outlined markers: geometric means. Lines join each benchmark’s endpoints.',
       'Shading: one sample standard deviation in log-ratio space across ten benchmarks, retaining covariance (n = 10).',
       'This is cross-benchmark spread, not run-to-run timing uncertainty or a confidence interval.',
       'Normalize to the measured cohort’s fewest LOC and fastest median. Apple M3 · AC power · Low Power Mode off.',
       '3 samples in broad comparisons; 7 in latest confirmations. Complete source LOC; generic libraries and compiler excluded.'])
    save(fig,'mech-ten.loc.stddev');plt.close(fig);return regions

def main():
    data=data_model();data['plot_checks']=mega(data);data['mech_spread']=mech_spread(data)
    (HERE/'ten-loc-endpoints.json').write_text(json.dumps(data,indent=2)+'\n')
    print('Exported white ten-benchmark charts',data['plot_checks'])

if __name__=='__main__':main()
