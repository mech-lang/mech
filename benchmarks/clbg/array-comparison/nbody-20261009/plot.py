"""White-background PNG: one compact/fastest line per tested language."""
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.ticker import FixedLocator, FuncFormatter
HERE=Path(__file__).resolve().parent
OUT=Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-png')
d=json.loads((HERE/'comparison-mech-compact.json').read_text())
assert d['status']=='complete'
p=d['programs']
assert set(p)=={'mech','mech-optimized','k','bqn','bqn-optimized'}
minimum_loc=min(r['normalized_loc'] for r in p.values())
minimum_time=min(r['median_seconds'] for r in p.values())
languages=[('Mech (AOT)','#DFA520',['mech','mech-optimized']),('K (Growler)','#8B668B',['k']),('BQN (CBQN)','#2B7067',['bqn','bqn-optimized'])]
colors={key:color for _,color,keys in languages for key in keys}
plt.rcParams.update({'font.family':'DejaVu Sans','font.size':12,'axes.spines.top':False,'axes.spines.right':False})
fig,ax=plt.subplots(figsize=(10,7))
fig.subplots_adjust(left=.13,right=.95,top=.76,bottom=.29)
ax.set_xscale('log',base=2);ax.set_yscale('log',base=2)
ax.set_xlim(.93,2.15);ax.set_ylim(.8,64)
ax.xaxis.set_major_locator(FixedLocator([1,2]))
ax.yaxis.set_major_locator(FixedLocator([1,2,4,8,16,32,64]))
fmt=FuncFormatter(lambda x,pos:f'{x:g}×')
ax.xaxis.set_major_formatter(fmt);ax.yaxis.set_major_formatter(fmt)
ax.minorticks_off();ax.grid(color='#E6E9EC',lw=.8);ax.set_axisbelow(True)
coords={key:(r['normalized_loc']/minimum_loc,r['median_seconds']/minimum_time) for key,r in p.items()}
for label,color,keys in languages:
    ax.plot([coords[k][0] for k in keys],[coords[k][1] for k in keys],color=color,lw=2.5,zorder=2)
for key,r in p.items():
    x,y=coords[key]
    ax.scatter([x]*len(r['samples']),[s['seconds']/minimum_time for s in r['samples']],s=23,color=colors[key],alpha=.24,zorder=2)
    ax.errorbar(x,y,yerr=r['sample_stdev_seconds']/minimum_time,color=colors[key],lw=1.5,capsize=4,zorder=3)
    ax.scatter(x,y,s=110,color=colors[key],edgecolor='white',lw=1.2,marker='s' if key.endswith('optimized') else 'o',zorder=4)
    name={'mech':'Mech compact','mech-optimized':'Mech optimized','k':'K','bqn':'BQN compact','bqn-optimized':'BQN optimized'}[key]
    offsets={'mech':(0,29),'mech-optimized':(22,16),'k':(7,27),'bqn':(0,24),'bqn-optimized':(-15,-28)}
    dx,dy=offsets[key]
    alignment='right' if key=='bqn-optimized' else ('left' if key in ('mech-optimized','k') else 'center')
    ax.annotate(f'{name}\n{r["normalized_loc"]} LOC · {r["median_seconds"]:.2f} s',(x,y),xytext=(dx,dy),textcoords='offset points',ha=alignment,va='center',color='#263238',fontsize=10.5,linespacing=1.4)
ax.set_xlabel('Source lines / fewest lines · log₂',labelpad=12)
ax.set_ylabel('Run time / fastest time · log₂',labelpad=12)
fig.text(.05,.95,'N-body: Mech, K, and BQN',fontsize=22,fontweight='bold')
fig.text(.05,.892,'50 million steps · Apple M3 · median of 5 sequential fresh-process runs',fontsize=12,color='#5A626B')
handles=[Line2D([],[],color=color,lw=2.5,label=name) for name,color,_ in languages]
fig.legend(handles=handles,loc='upper left',bbox_to_anchor=(.045,.855),ncol=3,frameon=False,handlelength=2,columnspacing=2)
fig.text(.05,.037,'Fewer lines left · faster below · both axes use log₂ scales. Circles: compact; squares: optimized.\n'
         'Lines join tested source endpoints; pale dots show all timing samples; bars show sample standard deviation.\n'
         'K keeps its fastest tested advance algorithm with shorter setup. K and BQN sources are local pure-language variants.\n'
         'One assignment/statement per counted line; matrix data keeps coordinate rows. Energy outputs pass the 1e−8 tolerance.\n'
         'Mech excludes its general repetition/output runner from LOC; K and BQN include those calls. Timing includes both.',fontsize=9.5,color='#5A626B',linespacing=1.6)
for ext in ('png','svg'):
    fig.savefig(HERE/('performance-loc.'+ext),dpi=180,facecolor='white')
    fig.savefig(OUT/('nbody-array-performance-loc.'+ext),dpi=180,facecolor='white')
(OUT/'nbody-array-performance.json').write_text(json.dumps(d,indent=2)+'\n')
plt.close(fig)
