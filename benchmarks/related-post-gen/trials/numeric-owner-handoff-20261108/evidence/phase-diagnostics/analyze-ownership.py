from pathlib import Path
import json,statistics as st
P=Path(__file__).parent/'profile';read=lambda p:json.loads(Path(p).read_text());rows=read(P/'profiles.json');summ=read(P/'aligned-phase-analysis.json');groups={};raw=[]
def d(e):
 out=dict(elapsed_ns=e['end_ns']-e['start_ns'])
 for f in ['instructions','cycles']:out[f]=e['end_counters'][f]-e['start_counters'][f]
 for f in ['thread_requests','thread_requested_bytes','thread_freed_bytes']:out[f]=e['end_allocations'][f]-e['start_allocations'][f]
 return out
for r in rows:
 if r['variant']not in ['handoff-enabled-detail','handoff-disabled-detail']:continue
 t=read(r['trace']);es=t['capture']['events'];selected={}
 for site in [844,1578,847,1581]:
  e=[e for e in es if e['stage']=='helper_ffi'and e['site']==site and e['operation']==2**64-27];assert len(e)==1;selected[str(site)]=d(e[0])
 selected['postings_combined']={k:selected['844'][k]+selected['1578'][k]for k in selected['844']}
 roots={str(site):[d(e)for e in es if e['stage']=='native_body'and e['site']==site]for site in [271,795,1036,1529]};assert all(len(v)==1 for v in roots.values());roots={k:v[0]for k,v in roots.items()};raw.append(dict(pair=r['pair'],variant=r['variant'],writes=selected,native_roots=roots));groups.setdefault(r['variant'],[]).append(raw[-1])
means={v:{part:{site:{k:st.median(x[part][site][k]for x in rs)for k in rs[0][part][site]}for site in rs[0][part]}for part in ['writes','native_roots']}for v,rs in groups.items()};paired={}
for name in list(summ['phases']['handoff-enabled-coarse']['phases']):
 data={v:{r['pair']:r['phases'][name]['elapsed_ns']for r in summ['runs']if r['variant']==v}for v in ['handoff-enabled-coarse','handoff-disabled-coarse','rust-fastest-coarse']}
 paired[name]={base:dict(median_difference_ns=st.median(data['handoff-enabled-coarse'][i]-data[base][i]for i in range(8)),wins=sum(data['handoff-enabled-coarse'][i]<data[base][i]for i in range(8)))for base in ['handoff-disabled-coarse','rust-fastest-coarse']}
out=dict(diagnostic_only=True,authoritative_timings_are_separate=True,source_and_native_Objects_unchanged=True,phase_medians_are_not_additive=True,native_root_includes_cold_helper_work=True,summary=means,paired_coarse_phase_changes=paired,raw=raw,already_implemented_geometry='numeric_words::linear_address already uses schema-certified unit dimensions; schema22 has Constant(1), so treating generic matrix division as a new saving for these arrays would be incorrect.')
(P/'ownership-profile-summary.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(dict(summary=means,paired_phases=paired),indent=2))
