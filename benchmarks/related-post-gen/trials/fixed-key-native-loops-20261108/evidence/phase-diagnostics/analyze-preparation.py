from pathlib import Path
import json,statistics,collections
P=Path(__file__).parent/'profile'
MAX=(1<<64)-1
rows=json.loads((P/'profiles.json').read_text())
summaries={}
for variant in ['canonical-control-detail','native-dictionary-detail']:
    runs=[]
    for row in rows:
        if row['variant']!=variant:continue
        trace=json.loads(Path(row['trace']).read_text())
        events=[e for e in trace['capture']['events']if e['thread']==trace['main_thread']]
        end=next(e for e in events if e['stage']=='coarse_checkpoint'and e['operation']==1)
        events=sorted([e for e in events if e['start_ns']<=end['start_ns']],key=lambda e:e['start_ns'])
        root=next(e for e in events if e['stage']=='post_JSON')
        helpers=[e for e in events if e['stage']=='helper_ffi']
        hist_begin=next(e for e in helpers if e['operation']==MAX-39)
        hist_end=next(e for e in helpers if e['operation']==MAX-40)
        lookup_begin=next(e for e in helpers if e['operation']==MAX-35)
        lookup_end=next(e for e in helpers if e['operation']==MAX-36)
        descriptor=json.loads((P/'hosts'/variant.removesuffix('-detail')/'build/descriptor.json').read_text())['descriptor']
        keysite=next(i for i,c in enumerate(descriptor['calls'])if c and c['template']['selected_key']==dict(module_path=['map'],operation_name='keys')and hist_begin['site']<i<lookup_begin['site'])
        keys=next(e for e in helpers if e['site']==keysite)
        def interval(a,b,sa='end',sb='start'):
            return dict(elapsed_ns=b[sb+'_ns']-a[sa+'_ns'],instructions=b[sb+'_counters']['instructions']-a[sa+'_counters']['instructions'],requests=b[sb+'_allocations']['thread_requests']-a[sa+'_allocations']['thread_requests'],requested_bytes=b[sb+'_allocations']['thread_requested_bytes']-a[sa+'_allocations']['thread_requested_bytes'])
        runs.append(dict(pair=row['pair'],parts=dict(
            before_histogram=interval(root,hist_begin,'start','start'),
            histogram_prepare=interval(hist_begin,hist_begin,'start','end'),
            histogram_native=interval(hist_begin,hist_end),
            histogram_finish=interval(hist_end,hist_end,'start','end'),
            before_keys=interval(hist_end,keys),
            keys=interval(keys,keys,'start','end'),
            dictionary_and_lookup_prelude=interval(keys,lookup_begin),
            lookup_prepare=interval(lookup_begin,lookup_begin,'start','end'),
            lookup_native=interval(lookup_begin,lookup_end),
            lookup_finish=interval(lookup_end,lookup_end,'start','end'),
            eligibility_and_prelude=interval(lookup_end,end))))
        total=interval(root,end,'start','start')
        assert all(sum(p[k]for p in runs[-1]['parts'].values())==total[k]for k in total)
    summaries[variant]=dict(parts={name:{k:statistics.median(r['parts'][name][k]for r in runs)for k in runs[0]['parts'][name]}for name in runs[0]['parts']},runs=runs)
(P/'preparation-analysis.json').write_text(json.dumps(dict(diagnostic_only=True,promotion_timing_eligible=False,all_intervals_disjoint_and_telescope=True,medians_not_additive=True,summaries=summaries),indent=2)+'\n')
for name,summary in summaries.items():
    print(name)
    for stage,v in summary['parts'].items():print(stage,round(v['elapsed_ns']/1e3,3),'us',v['instructions'],'instr',v['requests'],'alloc')
