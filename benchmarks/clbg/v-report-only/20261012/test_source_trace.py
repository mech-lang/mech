"""Authored, unexecuted bounded checks for work-trace rejection gates.

The standalone entry point owns the common timing mutex and requires the
root-owned oracle/preparation receipts before any fixture execution.
"""
from collections import Counter
import fcntl
import json
from pathlib import Path
import tempfile
import unittest

import full_study
import source_trace as trace

HERE=Path(__file__).resolve().parent
MUTEX_FD=None
WIDTHS=(1,2,3,4,6,12,18)
SEQUENCE='GGTATTTTAATTTATAGTACGT'
INPUT=b'>ONE\r\nACGT\r\n>THREEBAD\r\nAAAA\r\n;>THREE\r\n>THREE\tselected\r\nggtattttaatttatagta\r\n;>FOUR comment\r\ncgt\r\n>FOUR\r\nAAAA'
PI_STDOUT=b'31        \t:2\n'
PI_CANDIDATES=((3,4,1,None),(9,10,3,3),(0,10,3,None),(10,30,15,None),(110,170,105,1))


def map_fixture(variant,sequence=SEQUENCE):
    maps=[Counter(sequence[i:i+k] for i in range(max(0,len(sequence)-k+1))) for k in WIDTHS]
    lines=[f'TRACE_V1|knucleotide|{variant}|7|{len(sequence)}']
    for k,counts in zip(WIDTHS,maps):
        for word,value in sorted(counts.items()):
            key=word
            if variant=='optimized':
                packed=0
                for base in word:
                    packed=4*packed+'ACGT'.index(base)
                key=str(packed)
            lines.append(f'MAP|{k}|{key}|{value}')
        lines.append(f'TABLE|{k}|{len(counts)}|{sum(counts.values())}')
    lines.append('END_V1')
    output=[]
    for counts in maps[:2]:
        denominator=sum(counts.values())
        for word,count in sorted(counts.items(),key=lambda item:(-item[1],item[0])):
            output.append(f'{word} {100.0*count/denominator:.3f}\n')
        output.append('\n')
    for counts,query in zip(maps[2:],('GGT','GGTA','GGTATT','GGTATTTTAATT','GGTATTTTAATTTATAGT')):
        output.append(f'{counts[query]}\t{query}\n')
    return ('\n'.join(lines)+'\n').encode(),''.join(output).encode()


def pi_fixture(variant):
    lines=[f'TRACE_V1|pidigits|{variant}|2'];accepted=0
    roles=('lower','upper') if variant=='compact' else ('upper','lower')
    for candidate,(lower,upper,denominator,digit) in enumerate(PI_CANDIDATES,1):
        numerators=dict(lower=lower,upper=upper)
        for role in roles:
            numerator=numerators[role]
            lines.append(f'DIV|{candidate}|{role}|{numerator}|{denominator}|{numerator//denominator}')
        if digit is not None:
            accepted+=1
            lines.append(f'ACCEPT|{candidate}|{digit}|{accepted}')
    lines.append('END_V1')
    return ('\n'.join(lines)+'\n').encode()


class SourceTraceChecks(unittest.TestCase):
    def maps(self,variant='compact',payload=None,stdout=None,stdin=INPUT):
        reference,output=map_fixture(variant)
        return trace.reconcile_maps(reference if payload is None else payload,stdin,
            output if stdout is None else stdout,variant,common_timing_mutex_fd=MUTEX_FD)

    def pi(self,variant='compact',payload=None,stdout=PI_STDOUT,n=2):
        return trace.reconcile_pi(pi_fixture(variant) if payload is None else payload,
            stdout,n,variant,common_timing_mutex_fd=MUTEX_FD)

    def test_all_actual_source_anchor_copies(self):
        for benchmark,variants in trace.VARIANTS.items():
            for variant in sorted(variants):
                with self.subTest(benchmark=benchmark,variant=variant):
                    source=(HERE/'sources'/benchmark/(variant+'.v')).read_text()
                    amended,plan=trace.instrument_text(source,benchmark,variant)
                    # Original statements must remain byte-identical and ordered.
                    position=0
                    for line in source.splitlines():
                        if line:
                            position=amended.index(line,position)+len(line)
                    self.assertTrue(plan['never_chart_eligible'])
                    self.assertFalse(plan['full_performance_run'])

    def test_anchor_does_not_consume_blank_line(self):
        source='fn main() {\n    statement\n\n    next_statement\n}\n'
        amended=trace._insert_after(source,'statement','addition')
        self.assertIn('    statement\n    addition\n\n    next_statement',amended)

    def test_missing_or_duplicate_anchor_rejected(self):
        for source in ('nothing\n','statement\nstatement\n'):
            with self.subTest(source=source),self.assertRaises(AssertionError):
                trace._insert_before(source,'statement','addition')

    def test_reinstrumentation_rejected(self):
        source=(HERE/'sources/pidigits/compact.v').read_text()
        amended,_=trace.instrument_text(source,'pidigits','compact')
        with self.assertRaises(AssertionError):
            trace.instrument_text(amended,'pidigits','compact')

    def test_seven_compact_maps_comments_crlf_eof(self):
        result=self.maps()
        self.assertEqual(result['sequence_length'],len(SEQUENCE))
        self.assertEqual([row['k'] for row in result['complete_maps']],list(WIDTHS))
        self.assertTrue(result['first_two_maps_included'])

    def test_seven_packed_maps_reconcile(self):
        self.assertTrue(self.maps('optimized')['all_seven_maps_reconciled'])

    def test_empty_maps_and_stdout(self):
        payload,stdout=map_fixture('compact','')
        self.assertTrue(self.maps(payload=payload,stdout=stdout,stdin=b'>THREE')['all_seven_maps_reconciled'])

    def test_reader_only_accepts_space_tab_end_and_cr_lf(self):
        empty_payload,empty_stdout=map_fixture('compact','')
        for control in (b'\v',b'\f'):
            with self.subTest(control=control):
                self.assertTrue(self.maps(payload=empty_payload,stdout=empty_stdout,
                    stdin=b'>THREE'+control+b'invalid token\nACGT')['all_seven_maps_reconciled'])
                with self.assertRaises(AssertionError):
                    trace._sequence(b'>THREE\nAC'+control+b'GT')
        for newline in (b'\r',b'\n',b'\r\n'):
            with self.subTest(newline=newline):
                self.assertEqual(trace._sequence(b'>THREE'+newline+b'acgt'),'ACGT')

    def test_missing_length_one_table_rejected(self):
        payload,_=map_fixture('compact')
        lines=[line for line in payload.splitlines() if not line.startswith((b'MAP|1|',b'TABLE|1|'))]
        with self.assertRaises(AssertionError):self.maps(payload=b'\n'.join(lines)+b'\n')

    def test_duplicate_key_rejected(self):
        payload,_=map_fixture('compact');lines=payload.splitlines();lines.insert(2,lines[1])
        with self.assertRaises(AssertionError):self.maps(payload=b'\n'.join(lines)+b'\n')

    def test_wrong_packed_high_bits_rejected(self):
        payload,_=map_fixture('optimized');lines=payload.splitlines()
        fields=lines[1].split(b'|');fields[2]=b'4';lines[1]=b'|'.join(fields)
        with self.assertRaises(AssertionError):self.maps('optimized',payload=b'\n'.join(lines)+b'\n')

    def test_wrong_map_window_total_rejected(self):
        payload,_=map_fixture('compact')
        payload=payload.replace(f'TABLE|1|4|{len(SEQUENCE)}'.encode(),b'TABLE|1|4|999')
        with self.assertRaises(AssertionError):self.maps(payload=payload)

    def test_partial_or_duplicated_end_rejected(self):
        payload,_=map_fixture('compact')
        for malformed in (payload.replace(b'END_V1\n',b''),payload+b'END_V1\n'):
            with self.subTest(malformed=malformed[-30:]),self.assertRaises(AssertionError):self.maps(payload=malformed)

    def test_nucleotide_stdout_change_rejected(self):
        with self.assertRaises(AssertionError):self.maps(stdout=b'wrong\n')

    def test_all_pi_source_orders(self):
        for variant in ('compact','optimized','published-package'):
            with self.subTest(variant=variant):
                result=self.pi(variant)
                self.assertEqual(result['tested_candidates'],5)
                self.assertEqual(result['actual_divisions'],10)

    def test_zero_digit_trace(self):
        result=self.pi(payload=b'TRACE_V1|pidigits|compact|0\nEND_V1\n',stdout=b'',n=0)
        self.assertEqual(result['actual_divisions'],0)

    def test_missing_or_reordered_second_extract_rejected(self):
        lines=pi_fixture('compact').splitlines()
        missing=lines[:2]+lines[3:]
        swapped=lines.copy();swapped[1],swapped[2]=swapped[2],swapped[1]
        for malformed in (missing,swapped):
            with self.subTest(malformed=malformed[:3]),self.assertRaises(AssertionError):
                self.pi(payload=b'\n'.join(malformed)+b'\n')

    def test_wrong_denominator_or_quotient_rejected(self):
        payload=pi_fixture('compact')
        for malformed in (payload.replace(b'lower|3|1|3',b'lower|3|2|3'),payload.replace(b'lower|3|1|3',b'lower|3|1|4')):
            with self.subTest(malformed=malformed[:80]),self.assertRaises(AssertionError):self.pi(payload=malformed)

    def test_skipped_or_wrong_acceptance_rejected(self):
        payload=pi_fixture('compact')
        for malformed in (payload.replace(b'ACCEPT|2|3|1\n',b''),payload.replace(b'ACCEPT|2|3|1',b'ACCEPT|2|4|1')):
            with self.subTest(malformed=malformed[:120]),self.assertRaises(AssertionError):self.pi(payload=malformed)

    def test_extra_division_after_final_digit_rejected(self):
        payload=pi_fixture('compact').replace(b'END_V1',b'DIV|6|lower|0|1|0\nEND_V1')
        with self.assertRaises(AssertionError):self.pi(payload=payload)

    def test_pi_padding_count_and_complete_stdout_rejected(self):
        for stdout in (b'31\t:2\n',b'31        \t:3\n',PI_STDOUT+b'extra\n'):
            with self.subTest(stdout=stdout),self.assertRaises(AssertionError):self.pi(stdout=stdout)

    def test_invalid_or_unowned_mutex_descriptor_rejected(self):
        with tempfile.TemporaryFile() as unrelated:
            with self.assertRaises(AssertionError):
                trace.reconcile_pi(pi_fixture('compact'),PI_STDOUT,2,'compact',common_timing_mutex_fd=unrelated.fileno())
        with trace.LOCK.open('a+') as separate_description:
            with self.assertRaises(BlockingIOError):
                trace.reconcile_pi(pi_fixture('compact'),PI_STDOUT,2,'compact',common_timing_mutex_fd=separate_description.fileno())


def main():
    global MUTEX_FD
    print('WAIT LOCK: V bounded work-trace fixtures',flush=True)
    with trace.LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX);MUTEX_FD=lock.fileno()
        print('LOCKED: V bounded work-trace fixtures',flush=True)
        for name,count in (('oracle-fixtures.json',14),('preparation-fixtures.json',5),('cleanup-fixtures.json',8)):
            receipt=json.loads((HERE/name).read_text())
            assert receipt['status']=='passed' and receipt['tests_run']==count,(name,receipt)
        suite=unittest.defaultTestLoader.loadTestsFromTestCase(SourceTraceChecks)
        result=unittest.TextTestRunner(verbosity=2).run(suite)
        module=full_study.configure('nbody')
        destination=HERE/'work-trace-fixtures.json'
        assert not destination.exists(),'Never overwrite a retained work-trace fixture receipt'
        receipt=dict(status='passed' if result.wasSuccessful() else 'failed',tests_run=result.testsRun,
            failures=len(result.failures),errors=len(result.errors),created_at=module.now(),
            full_performance_runs=0,report_only=True,never_chart_eligible=True,
            shared_timing_lock=str(trace.LOCK),pins={str(path):module.sha(path) for path in (
                Path(__file__).resolve(),HERE/'source_trace.py',HERE/'oracle-fixtures.json',HERE/'preparation-fixtures.json',HERE/'cleanup-fixtures.json',
                *sorted((HERE/'sources/knucleotide').glob('*.v')),*sorted((HERE/'sources/pidigits').glob('*.v')),)})
        module.save(destination,receipt)
        if not result.wasSuccessful():raise SystemExit(1)


if __name__=='__main__':main()
