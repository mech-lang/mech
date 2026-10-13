"""Three retained-record/source corruption fixtures; no native subject."""
import copy
import json
import unittest

import native_reuse as reuse


class NativeReuse(unittest.TestCase):
    def test_exact_fifteen_profiles_126_records_preserved_without_relabelling(self):
        report=json.loads((reuse.PRIOR/'report.json').read_text())
        proposal=json.loads(reuse.PRIOR_PROPOSAL.read_text())
        original=copy.deepcopy(report)
        groups=reuse.validate_records(report,proposal['case_inventory'])
        self.assertEqual(len(groups),15)
        self.assertEqual(sum(len(group['records']) for group in groups.values()),126)
        self.assertEqual(report,original)
        self.assertNotIn('c3-binarytrees-optimized',groups)

    def test_missing_changed_nonboolean_native_or_partial_profile_record_rejects(self):
        actual=json.loads((reuse.PRIOR/'report.json').read_text())
        inventory=json.loads(reuse.PRIOR_PROPOSAL.read_text())['case_inventory']
        for mutation in ('missing','duplicate','identity','fingerprint','nonboolean','bool-wait','bool-ordinal','bool-workload','group','power','failure'):
            value=copy.deepcopy(actual)
            if mutation=='missing': value['attempts'].pop()
            elif mutation=='duplicate': value['attempts'][1]=copy.deepcopy(value['attempts'][0])
            elif mutation=='identity': value['attempts'][0]['id']='c3-binarytrees-optimized'
            elif mutation=='fingerprint': value['attempts'][0]['runtime_environment']['LC_ALL']='other'
            elif mutation=='nonboolean': value['attempts'][0]['qualification_checks']['pins_unchanged']=1
            elif mutation=='bool-wait': value['attempts'][0]['sampler_wait_status']=False
            elif mutation=='bool-ordinal': value['attempts'][0]['attempt']=True
            elif mutation=='bool-workload': value['attempts'][0]['workload_n']=True
            elif mutation=='group': value['attempts'][0]['supervised_native_group']['pgid']+=1
            elif mutation=='power': value['attempts'][0]['power_at_end']['low_power_mode']=1
            else: value['error']='unreviewed different failure'
            with self.subTest(mutation=mutation),self.assertRaises(AssertionError): reuse.validate_records(value,inventory)
        attempt=actual['attempts'][0]
        sampled=json.loads(reuse.Path(attempt['result_path']).read_text())
        enriched=json.loads(reuse.Path(attempt['supervisor_result_path']).read_text())
        reuse.validate_native_receipts(sampled,enriched,attempt)
        for target,field in (('enriched','sampler_wait_status'),('enriched','attempt'),('sampled','workload_n'),('enriched','workload_n')):
            raw,supervised=copy.deepcopy(sampled),copy.deepcopy(enriched)
            record=raw if target=='sampled' else supervised
            record[field]=False if field=='sampler_wait_status' else True
            with self.subTest(receipt=target,field=field),self.assertRaises(AssertionError):
                reuse.validate_native_receipts(raw,supervised,attempt)

    def test_only_five_explicit_widening_casts_in_three_unqualified_forms(self):
        reuse.validate_sources()
        old=json.loads((reuse.HERE/'revisions/private-v2/profiles.json').read_text())['profiles']
        new=json.loads((reuse.PACKET/'profiles.json').read_text())['profiles']
        changed=[before['id'] for before,after in zip(old,new) if before!=after]
        self.assertEqual(set(changed),{'c3-binarytrees-optimized','c3-fannkuchredux-compact','c3-fannkuchredux-optimized'})
        self.assertTrue(set(changed).isdisjoint(reuse.REUSED_IDS))


if __name__=='__main__': unittest.main()
