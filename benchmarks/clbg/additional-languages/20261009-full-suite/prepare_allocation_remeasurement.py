"""Reuse immutable valid measurements; replace only the rejected tree cohort."""
import hashlib,json
from pathlib import Path
from datetime import datetime,timezone
D=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def main():
    snapshot=D/'snapshots/pre-allocation-audit/results/timings.json'
    old=json.loads(snapshot.read_text());assert old['status']=='complete'
    audit=json.loads((D/'source-audit.json').read_text())
    admitted={r['id']:r for r in audit['rows']}
    kept={k:r for k,r in old['programs'].items() if r['benchmark']!='binarytrees'}
    assert len(kept)==57 and sum(len(r['samples_seconds']) for r in kept.values())==171
    for k,r in kept.items():
        assert sha(r['source'])==r['source_sha256'],k
        assert sha(r['binary'])==r['binary_sha256'],k
        if r['language']!='mech':
            current=admitted[k]
            assert r['source_sha256']==current['sha256'] and r['binary_sha256']==current['binary_sha256']
            assert r['loc']==current['normalized_loc'] and r['stdlib_files']==current['stdlib_files']
            for file in r['stdlib_files']:assert audit['libraries'][file]==old['library_pins'][file],file
    for file,digest in old['library_pins'].items():assert sha(D/file)==digest,file
    assert old['runtimes']==audit['runtimes']
    report=old|dict(status='partial',programs=kept,
        source_audit_sha256=sha(D/'source-audit.json'),library_pins=audit['libraries'],
        external_library_pins=audit['external_library_pins'],
        review_pins={f:sha(D/f) for f in ['profiles.json','results/builds.json','results/stdlib-tests.json','results/tree-work-audit.json','source-audit.json','run.py','prepare_allocation_remeasurement.py']},
        completed_benchmarks=[b for b in old['completed_benchmarks'] if b!='binarytrees'],
        orders={b:v for b,v in old['orders'].items() if b!='binarytrees'},
        unsupported=audit['unsupported'],
        reused_after_allocation_audit=dict(snapshot=str(snapshot.relative_to(D)),snapshot_sha256=sha(snapshot),
            at=datetime.now(timezone.utc).isoformat(),program_keys=sorted(kept),samples=171,
            unchanged_sources_binaries_and_used_libraries_verified=True,
            reason='All nine other workloads and their compiled binaries/support are unchanged. Five tree candidates fail the per-node allocation rule; they and the old tree control are removed. Corrected APR Mojo trees and current Mech control receive fresh full samples. Futhark tree allocation remains unsupported.'))
    report.pop('finished_at',None)
    (D/'results/timings.json').write_text(json.dumps(report,indent=2)+'\n')
    print('Verified and retained 171 valid samples across nine unchanged benchmarks; tree cohort removed for fresh measurements.')
if __name__=='__main__':main()
