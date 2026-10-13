"""Bounded additive-pool fixtures; no native execution or score publication."""
from copy import deepcopy
import json
from pathlib import Path
import tempfile
import unittest

import pending_minima as minima


def fixture(benchmark='mandelbrot'):
    identity,language=minima.STUDIES[benchmark]
    key=f'{benchmark}:{identity}'
    prior=dict(key=key,id=identity,benchmark=benchmark,language=language,
        original_loc=19,report='compact/manifest.json',
        source_files=[dict(path='/original/listed-source',sha256='a'*64)],
        aliases=['listed-alias'],samples=[],seconds=None)
    pool=dict(candidates=[dict(key='retained:measured',samples=[7.,8.,9.])],
        pending_candidates=[prior],report_paths=['old-report.json'],reconciliation=[])
    candidate=dict(id=identity,key=key,benchmark=benchmark,language=language,
        source='/frozen/measured-copy',source_sha256='a'*64,loc=None,
        samples_seconds=[3.,1.,2.],median_seconds=2.,
        sample_execution_metadata=[dict(attempt=i) for i in (1,4,5)])
    control=dict(id='mech-aot-control',key=benchmark+':loc-minimum-study:mech-control',
        benchmark=benchmark,language='mech',source='/frozen/exact-mech-control',
        source_sha256='b'*64,loc=None,samples_seconds=[4.,5.,6.],median_seconds=5.)
    return pool,{identity:candidate,'mech-aot-control':control}


class PendingMinimumPoolFixtures(unittest.TestCase):
    def merge(self,pool,records,benchmark='mandelbrot',discarded=()):
        return minima.merge_validated(pool,records,benchmark=benchmark,
            manifest_path='new/manifest.json',report_path='new/report.json',discarded=discarded)

    def test_all_four_move_exact_candidate_without_rewriting_producer_data(self):
        for benchmark in minima.STUDIES:
            with self.subTest(benchmark=benchmark):
                pool,records=fixture(benchmark)
                original=deepcopy(records)
                retained=deepcopy(pool['candidates'][0])
                row=self.merge(pool,records,benchmark)
                self.assertEqual(records,original)
                self.assertIn(retained,pool['candidates'])
                self.assertEqual(pool['pending_candidates'],[])
                self.assertIsNone(row['producer_loc'])
                self.assertEqual(row['original_source_pool_loc'],19)
                self.assertEqual(row['samples'],[3.,1.,2.])
                self.assertEqual(row['seconds'],2.)
                self.assertEqual(row['source_files'],[dict(path='/frozen/measured-copy',sha256='a'*64)])
                self.assertFalse(row['already_audited'])
                self.assertIsNone(row['authoritative_loc'])
                self.assertNotIn('normalized_loc',row)

    def test_fresh_control_remains_archived_without_becoming_a_candidate_or_relabel(self):
        pool,records=fixture()
        row=self.merge(pool,records,discarded=[dict(actual='environment-only')])
        self.assertEqual(row['fresh_paired_mech_control'],records['mech-aot-control'])
        self.assertNotIn('mech-aot-control',[p.get('id') for p in pool['candidates']])
        self.assertEqual(pool['reconciliation'][0]['control_timing_relabels'],0)
        self.assertFalse(pool['reconciliation'][0]['canonical_loc_admitted'])
        self.assertEqual(row['qualification']['discarded_environment_records'],1)

    def test_measured_copy_is_only_counted_source_and_original_is_provenance(self):
        pool,records=fixture()
        row=self.merge(pool,records)
        records[next(iter(records))]['samples_seconds'].append(99.)
        records['mech-aot-control']['source']='changed-later'
        self.assertEqual(row['samples'],[3.,1.,2.])
        self.assertEqual(len(row['source_files']),1)
        self.assertEqual(row['fresh_paired_mech_control']['source'],'/frozen/exact-mech-control')
        self.assertEqual(row['original_source_pool_report'],'compact/manifest.json')
        self.assertEqual(row['aliases'],['listed-alias'])

    def test_reject_missing_extra_wrong_profile_or_control_without_pool_mutation(self):
        for change in ('missing','extra','identity','control'):
            with self.subTest(change=change):
                pool,records=fixture();before=deepcopy(pool)
                identity=minima.STUDIES['mandelbrot'][0]
                if change=='missing':records.pop('mech-aot-control')
                elif change=='extra':records['extra']=dict(records[identity])
                elif change=='identity':records[identity]['language']='ruby'
                else:records['mech-aot-control']['benchmark']='other'
                with self.assertRaises(ValueError):self.merge(pool,records)
                self.assertEqual(pool,before)

    def test_reject_changed_source_missing_pending_short_samples_or_producer_loc(self):
        for change in ('source','pending','samples','loc'):
            with self.subTest(change=change):
                pool,records=fixture();identity=minima.STUDIES['mandelbrot'][0]
                if change=='source':records[identity]['source_sha256']='c'*64
                elif change=='pending':pool['pending_candidates']=[]
                elif change=='samples':records[identity]['samples_seconds'].pop()
                else:records[identity]['loc']=40
                before=deepcopy(pool)
                with self.assertRaises(ValueError):self.merge(pool,records)
                self.assertEqual(pool,before)

    def test_absent_and_incomplete_studies_never_call_validator(self):
        for status in ('absent','running','failed'):
            with self.subTest(status=status),tempfile.TemporaryDirectory() as folder:
                root=Path(folder);pool,_=fixture();before=deepcopy(pool)
                if status!='absent':
                    report=root/minima.COHORT/'mandelbrot/full-study/report.json'
                    report.parent.mkdir(parents=True)
                    report.write_text(json.dumps(dict(status=status)))
                def forbidden(*args,**kwargs):raise AssertionError('Incomplete study was admitted')
                minima.reconcile(root,pool,digest=forbidden,validate_study=forbidden)
                self.assertEqual(pool,before)

    def test_complete_reconciliation_uses_independent_adapter_and_exact_manifest_pin(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);pool,records=fixture();calls=[]
            base=root/minima.COHORT/'mandelbrot/full-study';base.mkdir(parents=True)
            manifest=dict(benchmark='mandelbrot',profiles=[dict(id=k,loc=None) for k in records],
                          pins={'/frozen/dependency':'d'*64})
            (base/'manifest.json').write_text(json.dumps(manifest))
            (base/'report.json').write_text(json.dumps(dict(status='complete')))
            def digest(path):return 'd'*64 if str(path)=='/frozen/dependency' else 'e'*64
            def validator(m,r,**kwargs):
                calls.append((m,r,kwargs));return records,[]
            minima.reconcile(root,pool,digest=digest,validate_study=validator)
            self.assertEqual(len(calls),1)
            self.assertEqual(calls[0][2]['manifest_sha256'],'e'*64)
            self.assertEqual(calls[0][2]['benchmark'],'mandelbrot')
            self.assertIs(calls[0][2]['digest'],digest)
            self.assertEqual(len(pool['candidates']),2)

    def test_pin_failure_stops_before_independent_validator(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);pool,records=fixture();before=deepcopy(pool)
            base=root/minima.COHORT/'mandelbrot/full-study';base.mkdir(parents=True)
            (base/'manifest.json').write_text(json.dumps(dict(benchmark='mandelbrot',
                profiles=[dict(id=k,loc=None) for k in records],pins={'/frozen/dependency':'d'*64})))
            (base/'report.json').write_text(json.dumps(dict(status='complete')))
            def forbidden(*args,**kwargs):raise AssertionError('Broken pin reached validator')
            with self.assertRaises(ValueError):
                minima.reconcile(root,pool,digest=lambda path:'c'*64,validate_study=forbidden)
            self.assertEqual(pool,before)


if __name__=='__main__':unittest.main()
