"""Three actual trace/reuse corruption fixtures; execute only under mutex."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import ownership_reuse_v7 as reuse
import source_proofs_v7 as corrected


class OwnershipTraceReuse(unittest.TestCase):
    def test_exact_prior_orc_success_and_failed_trace_remain_distinct(self):
        report=json.loads((reuse.PRIOR/'report.json').read_text())
        original=copy.deepcopy(report)
        passed,failed=reuse.validate_records(report)
        self.assertIs(passed['qualified'],True)
        self.assertIs(failed['qualified'],False)
        self.assertEqual(failed['source_work_error_type'],'NameError')
        self.assertEqual(report,original)
        for mutation in ('missing','orc-failed','failed-promoted','failure-name','nonboolean','native-group','power'):
            changed=copy.deepcopy(report)
            if mutation=='missing': changed['attempts'].pop()
            elif mutation=='orc-failed': changed['attempts'][0]['qualified']=False
            elif mutation=='failed-promoted': changed['attempts'][1]['qualified']=True
            elif mutation=='failure-name': changed['attempts'][1]['source_work_error']='different'
            elif mutation=='nonboolean': changed['attempts'][1]['qualification_checks']['pins_unchanged']=1
            elif mutation=='native-group': changed['attempts'][0]['supervised_native_group']['pgid']+=1
            else: changed['attempts'][0]['power_at_end']['low_power_mode']=1
            with self.subTest(mutation=mutation),self.assertRaises(AssertionError): reuse.validate_records(changed)

    def test_actual_saved_empty_trace_reconciles_seven_tables_without_proposal(self):
        path=reuse.PRIOR/'source-proofs/actual-seven-map-compact/empty/actual-maps.trace'
        original=path.read_text()
        result=corrected.reconcile(path,b'>THREE','compact')
        self.assertEqual(len(result['tables']),7)
        self.assertEqual(result['sequence_length'],0)
        self.assertTrue(all(value['keys']==value['windows']==0 for value in result['tables']))
        for mutation in (original.replace('TABLE|18|0|0\n',''),
                original.replace('TABLE|18|0|0','TABLE|17|0|0'),
                original.replace('TABLE|1|0|0','TABLE|1|1|0'),
                original.replace('END_NIM1','END_NIM1\nEND_NIM1')):
            with tempfile.TemporaryDirectory() as directory:
                changed=Path(directory)/'trace.txt';changed.write_text(mutation)
                with self.assertRaises(AssertionError): corrected.reconcile(changed,b'>THREE','compact')
        self.assertEqual(path.read_text(),original)
        report=json.loads((reuse.PRIOR/'report.json').read_text())
        self.assertIs(report['attempts'][1]['qualified'],False)

    def test_only_reconcile_local_initialization_changes_and_contracts_stay_exact(self):
        reuse.source_correction()
        source=Path(corrected.__file__).read_text()
        body=source[source.index('def reconcile('):source.index('\ndef run(')]
        self.assertNotIn('proposal[',body)
        self.assertIn('    summaries = []',body)


if __name__=='__main__': unittest.main()
