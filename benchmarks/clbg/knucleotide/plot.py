#!/usr/bin/env python3
"""Render complete validated k-nucleotide cohorts with gzip and LOC endpoints."""
import argparse,json,math
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter
COLORS={'chapel':'#188754','julia':'#9558b2','rust':'#aa5724','java':'#326994','go':'#008b9b','mech':'#e24b4b','c':'#606770','cpp':'#c39111'}
LABELS={'chapel':'Chapel','julia':'Julia','rust':'Rust','java':'Java (OpenJDK JIT)','go':'Go','mech':'Mech API prototype (new helpers excluded)','c':'C (GCC + LLVM)','cpp':'C++ (GCC + LLVM)'}
def render(path,data,metric,suffix,strict):
 rows=[r for r in data['programs'] if not strict or r['correctness_1000']['exact_reference_match']]
 revised=data.get('evaluation_status',{}).get('kind')=='complete_source_program'
 labels=LABELS|({'mech':'Mech (Cranelift AOT; complete source)'} if revised else {})
 comparison=bool(data.get('measurement_batches'))
 timing=lambda r:r['comparison_time_ratio'] if comparison else r['median_seconds']
 seconds=lambda r:r.get('comparison_median_seconds',r['median_seconds'])
 fastest_time=min(timing(r) for r in rows);smallest_size=min(r['source'][metric] for r in rows)
 fig,ax=plt.subplots(figsize=(11.5,7.6),layout='constrained');ax.set_xscale('log',base=2);ax.set_yscale('log',base=2)
 ax.xaxis.set_major_formatter(FuncFormatter(lambda x,_:f'{x:g}×'));ax.yaxis.set_major_formatter(FuncFormatter(lambda x,_:f'{x:g}×'));ax.grid(which='both',color='#dfe3e8',linewidth=.7);ax.set_axisbelow(True)
 table=['| Language | Fastest | Seconds | LOC | gzip bytes | Smallest | Seconds | LOC | gzip bytes | Time tradeoff |','| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |'];endpoints={}
 for language in sorted({r['language'] for r in rows}):
  cohort=[r for r in rows if r['language']==language];fast=min(cohort,key=lambda r:(timing(r),r['source'][metric]));small=min(cohort,key=lambda r:(r['source'][metric],timing(r)));color=COLORS[language]
  x=lambda r:r['source'][metric]/smallest_size;y=lambda r:timing(r) if comparison else timing(r)/fastest_time
  ax.scatter([x(r) for r in cohort],[y(r) for r in cohort],color=color,s=22,alpha=.25)
  failed=[r for r in cohort if not r['correctness_1000']['exact_reference_match']]
  if failed:ax.scatter([x(r) for r in failed],[y(r) for r in failed],color=color,marker='x',s=26,alpha=.6)
  ax.plot([x(fast),x(small)],[y(fast),y(small)],color=color,linewidth=1.6)
  ax.scatter(x(fast),y(fast),color=color,s=65,label=f"{labels[language]} ({len({r['path'] for r in cohort})} sources, {len(cohort)} builds)")
  if fast['id']!=small['id']:ax.scatter(x(small),y(small),color=color,marker='s',s=55)
  ids=[r['id']+('†' if not r['correctness_1000']['exact_reference_match'] else '') for r in (fast,small)]
  table.append(f"| {labels[language]} | {ids[0]} | {seconds(fast):.3f} | {fast['source']['nonblank_noncomment_lines']} | {fast['source']['gzip1_bytes']} | {ids[1]} | {seconds(small):.3f} | {small['source']['nonblank_noncomment_lines']} | {small['source']['gzip1_bytes']} | {timing(small)/timing(fast):.2f}× |")
  endpoints[language]={'fastest':fast['id'],'smallest':small['id'],'time_cost':timing(small)/timing(fast),'size_cost':fast['source'][metric]/small['source'][metric],'log2_line_length':math.hypot(math.log2(timing(small)/timing(fast)),math.log2(fast['source'][metric]/small['source'][metric]))}
  if language=='mech':
   if revised and len({r.get('native_object_sha256') for r in cohort})==1:
    ax.annotate(f"Mech: {small['source']['nonblank_noncomment_lines']} / {fast['source']['nonblank_noncomment_lines']} LOC\nSame native program · ~{seconds(fast):.2f}s",(x(small),y(small)),xytext=(12,18),textcoords='offset points',fontsize=9,color=color)
   else:
    ax.annotate(f"Fast Mech: {fast['source']['nonblank_noncomment_lines']} LOC\n{seconds(fast):.3f}s",(x(fast),y(fast)),xytext=(12,15),textcoords='offset points',fontsize=9,color=color)
    if fast['path']!=small['path']:ax.annotate(f"Small Mech: {small['source']['nonblank_noncomment_lines']} LOC\n{seconds(small):.3f}s",(x(small),y(small)),xytext=(12,-28),textcoords='offset points',fontsize=9,color=color)
 p=data['protocol'];unit='compressed program size' if metric=='gzip1_bytes' else 'nonblank, noncomment lines'
 title="k-nucleotide: execution time and complete source size" if revised else "k-nucleotide API experiment: time and call-site size"
 ax.set_title(f"{title}\n{data['hardware']['cpu']}; 125,000,000 letters; {p.get('samples_label') or 'median of '+str(p['samples'])+' fresh runs'}",loc='left',fontsize=15,pad=15)
 ax.set_xlabel(f'Normalized {unit} — smaller ←',fontsize=11);ax.set_ylabel('Elapsed time / same-batch Rust #7 — faster ↓' if comparison else 'Normalized elapsed time — faster ↓',fontsize=11);ax.legend(loc='upper right',fontsize=8.3,framealpha=.95)
 caption='Circle: fastest; square: smallest; faint points: all measured builds. Lines join each language’s two endpoints.\n'
 if comparison:caption+='Time uses the Rust #7 control in each batch; raw medians retained. Identical Mech executables/policies share pooled medians.\n'
 caption+='Same complete stdin and hash-table workload; startup/JIT and output included; AOT builds excluded.\n'
 caption+=('Mech includes source-defined frequency/query functions; generic sequence/map/IO library excluded. New opt-in compiler pathway.\n' if revised else 'Mech call-site size excludes new high-level helpers; this is not a fair cross-language implementation-size comparison.\n')
 caption+=('All pass both the official small test and the full benchmark.' if strict else 'All match the complete full-size output. Crosses/†: official small-test failure; see separate strict-cohort plot.')
 fig.text(.015,-.025,caption,fontsize=9,va='top',color='#424b57')
 stem=path.with_suffix('');target=stem.parent/(stem.name+suffix)
 for ext in ('png','svg'):fig.savefig(str(target)+'.'+ext,dpi=170,bbox_inches='tight')
 plt.close(fig);Path(str(target)+'.table.md').write_text('\n'.join(table)+'\n\n† Passes full benchmark output, fails official N=1000 test.\n'+('Seconds are raw batch medians, or pooled medians for identical Mech executable/policy pairs; endpoints and tradeoffs use same-batch Rust #7 ratios. See JSON for sample provenance.\n' if comparison else ''))
 Path(str(target)+'.endpoints.json').write_text(json.dumps({'source_report':path.name,'metric':metric,'strict_small_reference':strict,'time_baseline_seconds':None if comparison else fastest_time,'normalization':p.get('comparison_normalization'),'size_baseline':smallest_size,'languages':endpoints},indent=2)+'\n')
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('report',type=Path);a=p.parse_args();d=json.loads(a.report.read_text())
 for metric,suffix in [('gzip1_bytes',''),('nonblank_noncomment_lines','.loc')]:
  render(a.report,d,metric,suffix,False);render(a.report,d,metric,suffix+'.strict',True)
 print(a.report.with_suffix('.png'))
if __name__=='__main__':main()
