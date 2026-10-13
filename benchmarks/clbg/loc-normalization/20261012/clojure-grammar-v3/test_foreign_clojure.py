"""Unexecuted selected Clojure owner fixtures; no benchmark path special cases."""
import unittest

from foreign_clojure import Reader, normalize


class ClojureProposalFixtures(unittest.TestCase):
    def units(self, source, expected):
        result = normalize(source)
        self.assertEqual(result['diagnostics'], [], result['diagnostics'])
        self.assertEqual(result['normalized_loc'], expected)
        self.assertIs(result['proposal_only'], True)
        self.assertEqual(result['proposal_status'], 'unhooked-no-admission')
        return result

    def gate(self, source, kind):
        result = normalize(source)
        self.assertIn(kind, {item['kind'] for item in result['diagnostics']})
        self.assertEqual(result['review_status'], 'needs-review')
        return result

    def test_reader_positions_bom_crlf_comments_strings_regex_characters(self):
        source = '\ufeff; (fake)\r\n(def a "\\\"; ()[]{}\\\"")\r\n(def b #"[;()]+")\r\n(def c \\;) (def d \\))\r\n'
        reader = Reader(source)
        forms = reader.read()
        self.assertEqual(len(forms), 4)
        cleaned = ''.join(reader.cleaned)
        self.assertEqual(len(cleaned), len(source))
        self.assertEqual([i for i, char in enumerate(source) if char in '\r\n'],
                         [i for i, char in enumerate(cleaned) if char in '\r\n'])
        self.assertEqual(reader.diagnostics, [])
        self.units(source, 4)

    def test_recursive_reader_discard_removes_only_next_active_form(self):
        self.units('#_ #_ (def hidden 1) (def also-hidden 2) (def visible 3)', 1)
        self.gate('#_', 'foreign-clojure-discard-review')

    def test_packed_and_expanded_independent_definitions(self):
        self.units('(def a 1) (def b (+ a 1))', 2)
        self.units('(def a 1)\n(def b (+ a 1))', 2)

    def test_literal_data_wrapping_and_structural_commas_preserved(self):
        self.units('(def data [1, 2, {:x 3, :y 4}, #{5 6}])', 1)
        self.units('(def data [1\n  2\n  3])', 3)
        self.units("(def data '[(do (println 1) (println 2)) (fn [x] x)])", 1)

    def test_let_independent_initialized_bindings_packed_and_expanded(self):
        self.units('(let [a 1 b 2] (+ a b))', 5)
        self.units('(let [a 1\n b 2]\n (+ a b))', 5)

    def test_destructuring_one_producer_metadata_no_generated_bindings(self):
        self.units('(let [[a b] (vector 1 2) ^long c 3] (+ a b c))', 5)
        self.units('(defn f [^String x & values] (.length x))', 3)

    def test_loop_all_written_initialized_pairs_and_recur(self):
        self.units('(loop [i 0 total 0 step 1] (recur (+ i step) (+ total i) step))', 6)

    def test_do_body_effects_are_not_argument_lists(self):
        self.units('(do (println 1) (println 2) (println 3))', 5)
        self.units('(do\n (println 1)\n (println 2)\n (println 3))', 5)

    def test_explicit_fn_exposes_all_written_bodies(self):
        self.units('(fn [x] (println x) (inc x))', 4)

    def test_defn_doc_and_multiarity_body_owners(self):
        self.units('(defn f "written documentation" ^long [^long x] (+ x 1))', 3)
        self.units('(defn f ([x] x) ([x y] (println x) (+ x y)))', 9)

    def test_callback_bodies_inside_ordinary_call_arguments(self):
        self.units('(mapv (fn [x] (println x) (inc x)) [1 2])', 4)

    def test_callback_prefix_merge_preserves_effects_bindings_and_second_callbacks(self):
        self.units('(do (println 0) (mapv (fn [x] (println x) (inc x)) [1 2]))', 7)
        self.units('(let [a (fn [x] (println x) (inc x)) b 2] b)', 8)
        self.units('(mapv (fn [x] (println x) (inc x)) (fn [y] y))', 7)
        result = self.gate('(mapv (println 0) (fn [x] (println x) (inc x)))',
                           'foreign-clojure-callback-prefix-work-review')
        self.assertEqual(result['normalized_loc'], 5)

    def test_reify_method_owns_formals_body_and_written_closes(self):
        self.units('(reify Callable (call [_] (println 1) 2))', 6)

    def test_callable_reify_inside_submit_keeps_nested_method_effects(self):
        self.units('(.submit pool ^Callable (reify Callable (call [_] (println 1) 2)))', 6)

    def test_try_catch_finally_all_owned_body_sequences(self):
        self.units('(try (println 1) (catch Exception error (println error)) (finally (flush)))', 9)

    def test_if_when_and_condition_bindings_keep_header_ownership(self):
        self.units('(if (zero? x) 1 2)', 4)
        self.units('(when (< x 2) (println x) (inc x))', 4)
        self.units('(if-let [^String line (first lines)] (.length line) 0)', 4)

    def test_iteration_header_data_and_body_effects(self):
        self.units('(dotimes [i 3] (println i) (println (+ i 1)))', 4)
        self.units('(for [i (range 2) j (range 3)] [i j])', 3)
        self.units('(doseq [[word amount] rows] (println word amount))', 3)

    def test_cond_case_branch_owners_are_not_flat_data(self):
        self.units('(cond (< x 0) 1 :else 2)', 6)
        self.units('(case x 1 10 2 20 0)', 7)

    def test_java_interop_set_and_throw_expressions(self):
        self.units('(do (set! (.field object) 1) (.write System/out bytes))', 4)
        self.units('(throw (IllegalStateException. "broken"))', 1)

    def test_scalar_anonymous_composition_has_no_generated_param_units(self):
        self.units('#(+ %1 (* %2 2))', 1)

    def test_stateful_anonymous_callback_has_no_blanket_scalar_waiver(self):
        self.units('#(.get %)', 3)
        self.units('#(do (println %) (println (inc %)))', 4)

    def test_live_vector_argument_cannot_hide_nested_do_or_fn(self):
        self.units('(vector (do (println 1) (println 2)))', 4)
        self.units('(vector (fn [x] (println x) (inc x)))', 4)

    def test_written_namespace_requires_imports_directives_not_generated_code(self):
        self.units('(ns sample (:require [clbg.support :as s] [clojure.string :as string]) (:import [java.util Locale HashMap]) (:gen-class))', 7)
        self.gate('(ns sample (:import (java.util Locale HashMap)))', 'foreign-clojure-namespace-review')
        self.gate('(ns sample (:gen-class :methods [[run [] void]] :prefix (fn [x] (println x))))', 'foreign-clojure-generated-class-review')
        self.gate('(ns sample (:refer-clojure :exclude (fn [x] x)))', 'foreign-clojure-namespace-review')

    def test_unknown_macro_and_generated_scopes_require_review(self):
        self.gate('(unknown-macro [x 1] (println x))', 'foreign-clojure-call-owner-review')
        self.gate('(defmacro f [x] x)', 'foreign-clojure-call-owner-review')
        self.gate('(deftype Node [left right])', 'foreign-clojure-call-owner-review')
        self.gate('(comment (println 1))', 'foreign-clojure-call-owner-review')

    def test_reader_eval_tagged_conditionals_and_syntax_quotes_diagnosed(self):
        for source in ('#=(println 1)', '#custom {:x 1}', '#?(:clj 1)', '#:demo{:x 1}'):
            self.gate(source, 'foreign-clojure-dispatch-review')
        self.gate('`(let [x 1] ~x)', 'foreign-clojure-syntax-quote-review')

    def test_mismatched_unclosed_and_orphan_delimiters(self):
        self.gate('(def x [1 2))', 'foreign-clojure-delimiter-mismatch')
        self.gate('(def x [1 2]', 'foreign-clojure-unclosed-delimiter')
        self.gate(')', 'foreign-clojure-orphan-close')
        self.gate('(def x "missing)', 'foreign-clojure-unclosed-literal')

    def test_malformed_binding_metadata_and_generated_destructuring(self):
        self.gate('(let [x 1 y] x)', 'foreign-clojure-binding-vector-review')
        self.gate('(let [{:keys [x]} value] x)', 'foreign-clojure-destructuring-review')
        self.gate('^String', 'foreign-clojure-metadata-review')
        self.gate('(defn f [x] {:pre [(pos? x)]} x)', 'foreign-clojure-prepost-review')
        self.gate('(defn f {:inline (fn [x] (println x))} [x] x)', 'foreign-clojure-function-attributes-review')
        self.gate('^{:inline (fn [x] (println x))} (fn [x] x)', 'foreign-clojure-metadata-code-review')
        self.gate('(defn f ^{:inline (fn [x] (println x))} [x] x)', 'foreign-clojure-metadata-code-review')
        self.gate('(case)', 'foreign-clojure-control-signature-review')
        self.gate('(case (do (println 1) (println 2) 1) 1 10 0)', 'foreign-clojure-control-header-review')

    def test_unsupported_iteration_modifier_and_nested_anonymous(self):
        self.gate('(doseq [x rows :let [a 1 b 2]] (println a b))', 'foreign-clojure-iteration-modifier-review')
        self.gate('#(map #(+ % 1) %)', 'foreign-clojure-nested-anonymous-review')

    def test_loc_language_alias_is_rejected(self):
        for language in ('lisp', 'racket', 'scheme', 'java'):
            with self.assertRaises(ValueError):
                normalize('(def x 1)', language)

    def test_iteration_callback_header_keeps_callable_body_close_and_all_work(self):
        source = '(doseq [[word amount] (sort-by (fn [[word amount]] [(- amount) word]) table)] (println word amount))'
        result = self.units(source, 6)
        events = result['canonical_events']
        self.assertTrue(any(event['scope'] == 'foreign-clojure-doseq-callback-header' and event['kind'] == 'header' and event['units'] == 1 for event in events))
        self.assertTrue(any(event['scope'] == 'foreign-clojure-function' and event['kind'] == 'header' and event['units'] == 1 for event in events))
        self.assertTrue(any(event['scope'] == 'foreign-clojure-function' and event['kind'] == 'close' for event in events))
        self.assertTrue(all(left['end'] <= right['start'] for left, right in zip(events, events[1:])))
        self.units('(doseq [x (mapv (fn [a] a) (fn [b] b))] (println x))', 9)
        self.gate('(doseq [x (mapv (println 0) (fn [a] (println a) a))] (println x))',
                  'foreign-clojure-callback-prefix-work-review')
        self.gate('(doseq [x (do (println 0) rows)] (println x))', 'foreign-clojure-control-header-review')

    def test_typed_array_read_prefix_rejects_shadowed_and_effectful_operands(self):
        source = '(defn f [^doubles values column] (+ (aget values column) (if ok 1 2)))'
        result = self.units(source, 6)
        self.assertEqual(len(result['resolved_value_prefixes']), 1)
        for changed in (source.replace('column]', 'column aget]'),
                        source.replace('^doubles values', 'values'),
                        source.replace('(aget values column)', '(aget values (aset-double values 0 1.0))'),
                        '(def aget value) ' + source,
                        '(ns sample (:require [evil :refer [aget]])) ' + source,
                        '(defn f [^doubles values column] (let [values []] (+ (aget values column) (if ok 1 2))))'):
            self.gate(changed, 'foreign-clojure-callback-prefix-work-review')

    def test_exact_BigInteger_value_prefix_rejects_changed_helper_type_and_target(self):
        prefix = '(ns sample (:import [java.math BigInteger])) (defn integer [value] (BigInteger/valueOf (long value))) '
        body = '(loop [^BigInteger q BigInteger/ONE ^BigInteger r BigInteger/ZERO ^BigInteger t BigInteger/ONE] (let [^BigInteger three (.divide q t)] (vector (.multiply q (integer 10)) (.multiply (.subtract r (.multiply t three)) (integer 10)) (if ok q r))))'
        result = self.units(prefix + body, 18)
        self.assertEqual(len(result['resolved_value_prefixes']), 2)
        for changed in ((prefix + body).replace('java.math', 'java.util'),
                        (prefix + body).replace('[java.math BigInteger]', '[java.math BigInteger] [other BigInteger]'),
                        (prefix + body).replace('BigInteger/ONE', '(StringBuilder.)', 1),
                        (prefix + body).replace('(BigInteger/valueOf (long value))', '(do (println value) (BigInteger/valueOf (long value)))'),
                        (prefix + body).replace('(.multiply q', '(.append q', 1),
                        (prefix + body).replace('[^BigInteger three', '[integer unknown ^BigInteger three'),
                        prefix + '(def integer evil) ' + body,
                        prefix + '(defn integer [value] (println value) (BigInteger/valueOf (long value))) ' + body,
                        prefix.replace('(:import', '(:require [evil :refer [integer]]) (:import') + body):
            self.gate(changed, 'foreign-clojure-callback-prefix-work-review')
