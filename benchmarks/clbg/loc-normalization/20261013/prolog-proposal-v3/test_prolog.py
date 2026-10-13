"""Unexecuted original-reader/owner fixtures, separate from private-v1.

Root must freeze this file, scanner, projector, Python and exact SWI runtime;
the complete runner/scanner lifetime belongs under the common benchmark mutex.
PROLOG_SCANNER_RUNTIME must be an explicitly pinned binary, never PATH discovery.
PROLOG_SCANNER_LOCK_FD is the original common-lock owner's inherited descriptor.
No fixture loads a benchmark module or invokes a benchmark predicate.
"""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from foreign_prolog import BundleContext, count_prolog

HERE = Path(__file__).resolve().parent
HEADER = ':- module(m, []).\n'


class ReaderOwners(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        runtime = os.environ.get('PROLOG_SCANNER_RUNTIME')
        if not runtime or not Path(runtime).is_absolute():
            raise RuntimeError('Supply an explicit reviewed, pinned PROLOG_SCANNER_RUNTIME')
        cls.runtime = runtime
        cls.lock_fd = int(os.environ['PROLOG_SCANNER_LOCK_FD'])
        descriptor = os.fstat(cls.lock_fd)
        common = os.stat('/tmp/mech-clbg-timing.lock')
        if cls.lock_fd < 3 or (descriptor.st_dev, descriptor.st_ino) != (common.st_dev, common.st_ino):
            raise RuntimeError('Inherit the original owner common-lock descriptor')

    def scan(self, text, *, parser_error=False):
        with tempfile.TemporaryDirectory(prefix='prolog-owner-fixture-') as temporary:
            source = Path(temporary) / 'original.pl'
            source.write_bytes(text.encode('utf8'))
            return self.scan_path(source, text, parser_error=parser_error)

    def scan_path(self, source, text, *, parser_error=False):
        command = [self.runtime, '-q', '-f', 'none', '-l', str(HERE / 'scan_source.pl'),
                   '-g', 'prolog_source_scan_v1:main', '--', str(source)]
        completed = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   check=False, timeout=20, pass_fds=(self.lock_fd,))
        if parser_error:
            self.assertNotEqual(completed.returncode, 0)
            self.assertTrue(completed.stderr)
            return None
        self.assertEqual(completed.returncode, 0, completed.stderr.decode('utf8', errors='replace'))
        packet = json.loads(completed.stdout)
        self.assertEqual(packet['source_text'], text)
        self.assertEqual(packet['version'], [10, 0, 2])
        return packet

    def project(self, body, expected=None, *, language='prolog'):
        text = HEADER + body
        packet = self.scan(text)
        result = count_prolog(text.encode('utf8'), packet, language=language,
                              context=BundleContext([packet]))
        if expected is None:
            self.assertIsNone(result['normalized_loc'])
            self.assertTrue(result['diagnostics'])
        else:
            self.assertEqual(result['diagnostics'], [])
            self.assertEqual(result['normalized_loc'], expected)
            events = result['canonical_events']
            for left, right in zip(events, events[1:]):
                self.assertLessEqual(left['end'], right['start'])
            for event in events:
                self.assertEqual(text[event['start']:event['end']].encode('utf8'),
                    text.encode('utf8')[event['byte_start']:event['byte_end']])
        self.assertFalse(result['chart_admission'])
        return result

    def test_each_fact_keeps_clause_dispatch(self):
        self.project('p(0). p(1).\n', 3)

    def test_formal_and_data_commas_are_structural(self):
        self.project('p([A,B], pair(A,B)).\n', 2)

    def test_packed_unification_arithmetic_cut_are_independent(self):
        result = self.project('p(A,B) :- A=1, B is A+2, !.\n', 6)
        self.assertEqual([event['kind'] for event in result['canonical_events'] if event['units']],
            ['directive-header', 'clause-dispatch-header', 'unification',
             'arithmetic-binding', 'cut', 'written-clause-close'])

    def test_executable_packing_does_not_change_units(self):
        packed = self.project('p(A,B) :- A=1,B=2,true.\n', 6)
        expanded = self.project('p(A,B) :-\n A=1,\n B=2,\n true.\n', 6)
        self.assertNotEqual(packed['physical_loc'], expanded['physical_loc'])

    def test_decimal_and_exponent_dots_are_not_terminators(self):
        self.project('p(X) :- X is 1.25e-3 + 0.5.\n', 4)

    def test_literal_and_comment_punctuation_is_not_executable(self):
        self.project("% . :- , ; ->\np :- format('a. ; , -> /* % */ \\'', []). /* . , */\n", 4)

    def test_if_then_else_has_condition_arm_and_written_close_owners(self):
        result = self.project('p(X) :- (X > 0 -> X=1 ; X=2).\n', 8)
        merged = [event for event in result['canonical_events'] if event.get('merged_with_containing_control_header')]
        self.assertEqual([event['kind'] for event in merged], ['comparison'])
        self.assertEqual(sum(event['kind'] == 'written-control-close' for event in result['canonical_events']), 1)

    def test_hidden_condition_binding_and_second_test_are_retained(self):
        result = self.project('p(X) :- (X=1, X>0 -> true ; fail).\n', 9)
        conditions = [event for event in result['canonical_events'] if event['kind'] in {'unification', 'comparison'}]
        self.assertEqual([event['units'] for event in conditions], [0, 1])

    def test_nested_disjunctive_condition_and_right_associated_if_chain(self):
        result = self.project('p(X) :- ((X=1 ; X=2) -> true ; X=3 -> fail ; true).\n', 15)
        self.assertEqual(sum(event['kind'] == 'control-header' for event in result['canonical_events']), 2)
        self.assertEqual(sum(event['kind'] == 'branch-header' for event in result['canonical_events']), 3)
        self.assertTrue(all(event['units'] == 1 for event in result['canonical_events']
                            if event['kind'] in {'control-header', 'branch-header'}))

    def test_nested_negation_and_meta_condition_headers_stay_independent(self):
        result = self.project('p :- (\\+ true -> true ; fail).\n', 9)
        self.assertTrue(all(event['units'] == 1 for event in result['canonical_events']
                            if event['kind'] in {'control-header', 'branch-header'}))
        result = self.project('p :- (catch(true, Error, throw(Error)) -> true ; fail).\n', 12)
        self.assertTrue(all(event['units'] == 1 for event in result['canonical_events']
                            if event['kind'] == 'meta-call-header'))

    def test_vendor_meta_requires_selected_import_and_exact_target(self):
        callback = 'count_match(_, A, A).\n'
        call = 'p :- re_foldl(count_match, "a", "a", 0, _, []).\n'
        self.project(callback + call)
        self.project(callback + call.replace('re_foldl(', 'pcre:re_foldl('))
        selected = ':- use_module(library(pcre), [re_foldl/6]).\n'
        accepted = self.project(selected + callback + call, 7)
        references = [event for event in accepted['canonical_events']
                      if event['kind'] == 'named-closure-reference']
        self.assertEqual(len(references), 1)
        self.assertEqual(references[0]['units'], 0)
        self.assertEqual(references[0]['resolved_target'], ('m', 'count_match/3'))
        self.project(selected + callback + call.replace('re_foldl(', 'hashtable:re_foldl('))
        self.project(selected + callback + call.replace('count_match,', 'absent_match,'))
        self.project(':- use_module(library(pcre), [re_replace/5]).\n' + callback +
                     call.replace('re_foldl(', 'pcre:re_foldl('))

    def test_disjunction_does_not_turn_arms_into_argument_data(self):
        self.project('p :- (true ; fail).\n', 7)

    def test_catch_written_goal_and_recovery_bodies_are_exposed(self):
        self.project('p(X) :- catch((X=1,true), Error, throw(Error)).\n', 9)

    def test_forwarded_meta_formal_is_a_reference_without_invented_body(self):
        result = self.project('run(G) :- call(G). main :- run(true).\n', 10)
        references = [event for event in result['canonical_events'] if event['kind'] == 'formal-meta-reference']
        self.assertEqual(len(references), 1)
        self.assertEqual(references[0]['units'], 0)

    def test_direct_variable_goal_keeps_its_written_meta_call(self):
        result = self.project('run(G) :- G. main :- run(true).\n', 9)
        direct = [event for event in result['canonical_events'] if event['kind'] == 'variable-meta-call']
        self.assertEqual(len(direct), 1)
        self.assertEqual(direct[0]['units'], 1)

    def test_literal_data_wrapping_is_preserved(self):
        result = self.project('p([1,\n 2,\n 3]).\n', 4)
        self.assertEqual(sum(event['kind'] == 'literal-data-continuation' for event in result['canonical_events']), 2)

    def test_ordinary_call_argument_wrapping_does_not_expand(self):
        self.project('p(X) :- arg(1,\n X,\n 3).\n', 4)

    def test_utf8_character_positions_are_reconciled_to_original_bytes(self):
        result = self.project("p :- format('é . ; % ~n', []).\n", 4)
        close = result['canonical_events'][-1]
        self.assertEqual(close['byte_start'], close['start'] + 1)

    def test_input_initialization_is_read_as_data_and_never_invoked(self):
        self.project(':- initialization(throw(would_execute_input), main). p.\n', 6)

    def test_unknown_call_with_hidden_meta_body_is_gated(self):
        self.project('p :- unknown((true,fail)).\n')

    def test_local_constructed_meta_target_is_gated(self):
        self.project('p :- G=true, call(G).\n')

    def test_binding_a_forwarded_meta_formal_is_gated(self):
        self.project('run(G) :- G=true, call(G).\n')

    def test_helper_import_requires_its_actual_source_path(self):
        with tempfile.TemporaryDirectory(prefix='prolog-helper-fixture-') as temporary:
            root = Path(temporary)
            main_text = HEADER + ":- use_module('helper'). p :- go.\n"
            helper_text = ':- module(helper, [go/0]). go :- true.\n'
            (root / 'original.pl').write_bytes(main_text.encode('utf8'))
            (root / 'helper.pl').write_bytes(helper_text.encode('utf8'))
            main = self.scan_path(root / 'original.pl', main_text)
            helper = self.scan_path(root / 'helper.pl', helper_text)
            accepted = count_prolog(main_text.encode('utf8'), main, language='prolog',
                                     context=BundleContext([main, helper]))
            self.assertEqual(accepted['diagnostics'], [])
            self.assertEqual(accepted['normalized_loc'], 5)
            wrong = copy.deepcopy(helper)
            wrong['source'] = str(root / 'unbound' / 'helper.pl')
            rejected = count_prolog(main_text.encode('utf8'), main, language='prolog',
                                     context=BundleContext([main, wrong]))
            self.assertIsNone(rejected['normalized_loc'])

    def test_unknown_unused_helper_import_is_gated(self):
        self.project(":- use_module('unbound'). p.\n")

    def test_same_path_context_requires_exact_source_ast_and_reader_settings(self):
        with tempfile.TemporaryDirectory(prefix='prolog-context-fixture-') as temporary:
            source = Path(temporary) / 'original.pl'
            old_text = HEADER + 'q. p :- q.\n'
            source.write_bytes(old_text.encode('utf8'))
            old = self.scan_path(source, old_text)
            fresh_text = HEADER + 'p :- q.\n'
            source.write_bytes(fresh_text.encode('utf8'))
            fresh = self.scan_path(source, fresh_text)
            rejected = count_prolog(fresh_text.encode('utf8'), fresh, language='prolog',
                                     context=BundleContext([old]))
            self.assertIsNone(rejected['normalized_loc'])
            self.assertTrue(any('exact bundle context receipt' in issue.get('detail', '')
                                for issue in rejected['diagnostics']))
            stable_text = HEADER + 'q. p :- q.\n'
            source.write_bytes(stable_text.encode('utf8'))
            stable = self.scan_path(source, stable_text)
            altered_ast = copy.deepcopy(stable)
            altered_ast['forms'][1]['node']['value'] = 'renamed_q'
            altered_settings = copy.deepcopy(stable)
            altered_settings['reader_settings']['double_quotes'] = 'atom'
            for changed_context in (altered_ast, altered_settings):
                result = count_prolog(stable_text.encode('utf8'), stable, language='prolog',
                                      context=BundleContext([changed_context]))
                self.assertIsNone(result['normalized_loc'])
                self.assertTrue(any('exact bundle context receipt' in issue.get('detail', '')
                                    for issue in result['diagnostics']))
            vendor_text = HEADER + ':- use_module(library(pcre), [re_replace/5]).\n' + \
                'count_match(_, A, A). p :- re_foldl(count_match, "a", "a", 0, _, []).\n'
            source.write_bytes(vendor_text.encode('utf8'))
            caller_packet = self.scan_path(source, vendor_text)
            frozen_receipt = copy.deepcopy(caller_packet)
            context = BundleContext([caller_packet])
            indicator = caller_packet['forms'][1]['node']['children'][0]['children'][1]['children'][0]
            indicator['children'][0]['value'] = 're_foldl'
            indicator['children'][1]['value'] = '6'
            result = count_prolog(vendor_text.encode('utf8'), frozen_receipt, language='prolog',
                                  context=context)
            self.assertIsNone(result['normalized_loc'])
            self.assertTrue(any(issue['kind'] == 'prolog-unproved-call-target'
                                for issue in result['diagnostics']))
            self.assertFalse(any('exact bundle context receipt' in issue.get('detail', '')
                                 for issue in result['diagnostics']))

    def test_operator_directive_is_not_applied(self):
        self.scan(HEADER + ':- op(700,xfx,custom). p :- a custom b.\n', parser_error=True)

    def test_supported_reader_syntax_is_not_unsupported_grammar_admission(self):
        self.project(':- op(700,xfx,custom). p.\n')
        self.project('p --> [a].\n')
        self.project('p(_{key:value}).\n')
        self.project('p({|unknown_handler||payload|}).\n')

    def test_missing_full_stop_is_a_reader_error(self):
        self.scan(HEADER + 'p :- true\n', parser_error=True)

    def test_literal_end_of_file_cannot_hide_unread_source(self):
        self.project('end_of_file. p.\n')

    def test_perl_language_does_not_select_prolog_for_pl_extension(self):
        self.project('p.\n', language='perl')

    def test_mismatched_original_bytes_and_changed_operator_inventory_are_gated(self):
        text = HEADER + 'p.\n'
        packet = self.scan(text)
        context = BundleContext([packet])
        mismatched = count_prolog((text + '% changed\n').encode('utf8'), packet,
                                  language='prolog', context=context)
        self.assertIsNone(mismatched['normalized_loc'])
        changed = copy.deepcopy(packet)
        changed['operators_after'] = []
        result = count_prolog(text.encode('utf8'), changed, language='prolog', context=context)
        self.assertIsNone(result['normalized_loc'])
        generated = copy.deepcopy(packet)
        generated['source'] = generated['source'].removesuffix('.pl') + '.wam'
        result = count_prolog(text.encode('utf8'), generated, language='prolog',
                              context=BundleContext([generated]))
        self.assertIsNone(result['normalized_loc'])


if __name__ == '__main__':
    unittest.main()
