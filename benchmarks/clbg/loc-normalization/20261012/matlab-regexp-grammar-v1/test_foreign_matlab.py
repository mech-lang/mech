"""Source-only meaningful fixtures; execute only under the common mutex."""
import unittest

from foreign_matlab import mask_matlab, normalize


class MatlabRegexpFixtures(unittest.TestCase):
    def count(self, source, expected):
        metric = normalize(source, 'matlab')
        self.assertEqual(metric['normalized_loc'], expected, metric)
        self.assertEqual(metric['diagnostics'], [], metric)
        self.assertTrue(metric['proposal_only'])
        self.assertFalse(metric['eligible_for_chart'])
        return metric

    def gate(self, source, kind):
        metric = normalize(source, 'matlab')
        self.assertTrue(any(item['kind'] == kind for item in metric['diagnostics']), metric)
        self.assertEqual(metric['review_status'], 'needs-review', metric)
        self.assertFalse(metric['eligible_for_chart'])
        return metric

    def test_function_headers_parameters_and_written_closes(self):
        self.count('function f(x,y)\nemit(x,y);\nend', 3)
        metric = self.count('function [a,b]=f(x,y)\na=x; b=y;\nend', 4)
        self.assertEqual([e['units'] for e in metric['canonical_events'] if e['kind']=='header'], [1])
        self.count('function f\nwork();\nend', 3)

    def test_for_inline_expanded_and_nested_owners(self):
        for source in ('function f(); for j=1:size(data,1); emit(j); end; end',
                       'function f()\nfor j=1:size(data,1)\nemit(j);\nend\nend'):
            self.count(source, 5)
        self.count('for a=1:n, for b=1:m, emit(a,b), end, end', 5)

    def test_actual_char_read_transpose_and_nonconjugate_transpose(self):
        source="raw=fread(fid,Inf,'*char')'; other=raw.';"
        masked = mask_matlab(source)
        self.assertEqual(masked['diagnostics'], [])
        self.assertEqual(len(masked['cleaned']), len(source))
        self.assertEqual([item['kind'] for item in masked['transposes']], ['conjugate', 'nonconjugate'])
        self.assertEqual(len(masked['literals']), 1)
        self.count(source, 2)

    def test_quoted_regex_text_and_doubled_quotes_keep_exact_spans(self):
        source = "x='for end ; , % ''quote'' [ ]'; y=\"case ; % \"\"quoted\"\"\";"
        masked = mask_matlab(source)
        self.assertEqual(masked['diagnostics'], [])
        self.assertEqual(len(masked['cleaned']), len(source))
        self.assertEqual(len(masked['literals']), 2)
        self.count(source, 2)

    def test_line_and_alone_block_comments_do_not_own_code(self):
        source="% end function\nx=1; % for j=1:n\n%{\nfor fake=1:n; hidden(); end\n%}\ny=2;"
        self.count(source, 2)
        self.count("x=1; %{ this is a line comment\ny=2;", 2)
        self.gate('%{\nx=1;', 'matlab-unclosed-block-comment')

    def test_array_matrix_and_argument_separators_are_structural(self):
        self.count('A=[1,2;3,4]; emit(A(1,2),A(2,1));', 2)
        self.count('patterns=["a|b" "c[gt]"]; substitutions=["x" "<1>";"y" "-"];', 2)
        self.count("values={'a','b';'c','d'};", 1)

    def test_wrapped_literal_data_rows_are_preserved(self):
        self.count('A=[\n1 2;\n3 4\n];', 4)
        self.count('patterns=[\n"first"\n"second"\n];', 4)

    def test_command_comma_and_semicolon_expose_independent_work(self):
        self.count('a=1,b=2;emit(a,b),clear a;', 4)
        metric = self.count('function f(),a=1;emit(a),end', 4)
        self.assertTrue(any(e['scope']=='matlab-command-separator' and e['units']==0 for e in metric['canonical_events']))

    def test_initialized_bindings_clear_and_field_writes_are_work(self):
        self.count('x=read(); y=x; clear x; state.value=y;', 4)
        self.gate('[a,b]=read();', 'matlab-output-binding-review')
        self.gate('x=(state=1);', 'matlab-multiple-assignment-review')

    def test_index_end_does_not_close_a_function_or_loop(self):
        metric = self.count('function f(A)\nx=A(end-1);\nfor j=1:numel(A)\nx=A(j);\nend\nend', 6)
        self.assertEqual(len([e for e in metric['canonical_events'] if e['kind']=='close']), 2)

    def test_function_signature_and_iterator_cannot_hide_effect(self):
        self.gate('function f(x=work())\nend', 'matlab-function-header-review')
        self.gate('for j=read(state=1); emit(j); end', 'matlab-for-header-review')
        self.gate('function f(x) work();\nend', 'matlab-function-header-review')

    def test_array_owner_cannot_hide_assignment_call_or_callback(self):
        for source in ('A=[1;state=2];', 'A=[emit();emit()];', 'A=[@(x) emit(x)];'):
            self.gate(source, 'matlab-array-effect-owner-review')

    def test_callbacks_and_dynamic_readers_are_diagnostic(self):
        self.gate('f=@(x) emit(x);', 'matlab-callback-owner-review')
        self.gate('f=@helper;', 'matlab-callback-owner-review')
        for source in ("eval('for j=1:2;emit(j);end');", "feval('helper',x);", "run('other.m');", "f=str2func('helper');"):
            self.gate(source, 'matlab-dynamic-reader-review')
        self.gate('!other_program', 'matlab-external-reader-review')

    def test_unsupported_controls_and_malformed_owners_are_gated(self):
        self.gate('if ok; emit(); else; failed(); end', 'matlab-unsupported-control-review')
        self.gate('function f()\nfor j=1:3\nemit(j);\nend', 'matlab-unclosed-keyword-scope')
        self.gate('end', 'matlab-unmatched-keyword-end')
        self.gate('x=read(1;', 'matlab-unclosed-delimiter')
        self.gate('x=read(for j=1:2);', 'matlab-keyword-expression-review')

    def test_unselected_continuation_ambiguous_quote_and_unclosed_text_gate(self):
        self.gate('x=1+... ignored end\n2;', 'matlab-continuation-scope-review')
        self.gate("x=A ';", 'matlab-quote-context-review')
        self.gate("x='unterminated\ny=2;", 'matlab-unclosed-text-literal')
        self.gate('%{\n%{\n%}\n', 'matlab-nested-block-comment-review')

    def test_selected_regex_shapes_keep_both_loops_and_all_body_work(self):
        source="""function regex_example()
fid=fopen('/dev/fd/3','rb');raw=fread(fid,Inf,'*char')';fclose(fid);
dna=regexprep(raw,'>[^\\n]*\\n|\\n','');
patterns=["agggtaaa|tttaccct" "[cgt]gggtaaa|tttaccc[acg]"];
for pattern=patterns
fprintf('%s %d\\n',pattern,numel(regexp(dna,pattern,'start')));
end
substitutions=["tHa[Nt]" "<4>";"a[NSt]|BY" "<2>"];
for j=1:size(substitutions,1)
dna=regexprep(dna,substitutions(j,1),substitutions(j,2));
end
fprintf('\\n%d\\n',numel(dna));
end"""
        metric = self.count(source, 15)
        self.assertEqual(len([s for s in metric['scope_evidence'] if s['kind']=='for']), 2)
        self.assertTrue(all(s['end'] is not None for s in metric['scope_evidence']))
        self.count(source.replace('for pattern=patterns\n', 'clear raw;\nfor j=1:numel(patterns)\nstarts=regexp(dna,patterns(j),\'start\');\n')
            .replace("fprintf('%s %d\\n',pattern,numel(regexp(dna,pattern,'start')));", "fprintf('%s %d\\n',patterns(j),numel(starts));"), 17)


if __name__ == '__main__':
    unittest.main()
