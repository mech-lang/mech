"""Ensure new languages preserve the earlier source and timing measurements."""
import argparse,json,hashlib
from pathlib import Path

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--previous',required=True,type=Path)
    parser.add_argument('--current',type=Path,default=Path(__file__).parent/'dist/data.json')
    parser.add_argument('--report',type=Path)
    parser.add_argument('--added-languages',nargs='+',default=['matlab'])
    args=parser.parse_args()
    old=json.loads(args.previous.read_text());new=json.loads(args.current.read_text())
    old_keys={l['key'] for l in old['languages']};new_keys={l['key'] for l in new['languages']}
    assert new_keys-old_keys==set(args.added_languages) and old_keys<=new_keys
    for key,program in old['programs'].items():
        assert key in new['programs'],key
        # Ratios may change if a new source establishes a smaller measured minimum.
        before={k:v for k,v in program.items() if k not in ('x','y')}
        after={k:v for k,v in new['programs'][key].items() if k not in ('x','y')}
        assert before==after,('Existing program changed',key)
    for key,file in old['files'].items():assert file==new['files'][key],('Existing source changed',key)
    for bench in old['benchmarks']:
        current=next(b for b in new['benchmarks'] if b['key']==bench['key'])
        assert current['baseline_loc']<=bench['baseline_loc']
        assert current['baseline_seconds']<=bench['baseline_seconds']
        for group in bench['groups']:
            assert group==next(g for g in current['groups'] if g['language']==group['language'])
    for program in new['programs'].values():
        assert program['x']>=1
        if program['y'] is not None:assert program['y']>=1
    report=dict(status='passed',previous_sha256=hashlib.sha256(args.previous.read_bytes()).hexdigest(),
                current_sha256=hashlib.sha256(args.current.read_bytes()).hexdigest(),
                preserved_endpoint_programs=len(old['programs']),preserved_source_files=len(old['files']),
                preserved_timing_samples=sum(len(p['samples']) for p in old['programs'].values()),
                new_languages=args.added_languages,new_endpoint_programs=len(new['programs'])-len(old['programs']))
    if args.report:args.report.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report))

if __name__=='__main__':main()
