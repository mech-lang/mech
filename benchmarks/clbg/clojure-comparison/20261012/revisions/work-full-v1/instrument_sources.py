"""Unexecuted exact-text observer edits; invokes original tree class methods.

No subject import, reader evaluation, Java executor, source writes or hashing.
The mutex-held work controller supplies retained exact original source bytes.
"""


def once(source, old, new, edits, role):
    assert source.count(old) == 1, ('Reviewed observer insertion must match exactly once', role, old)
    edits.append(dict(role=role, original=old, observed=new, expected_occurrences=1))
    return source.replace(old, new, 1)


def support(source):
    edits = []
    source = once(source, '(ns clbg.support\n', '(ns clbg.work.support\n  (:require [clbg.work.observer :as o])\n', edits, 'separate-observer-helper-namespace')
    source = once(source, '(fn [function]', '(fn [worker function]', edits, 'actual-task-index')
    source = once(source, '(call [_] (function))', '(call [_] (o/with-worker worker function))', edits, 'actual-task-owner-binding')
    source = once(source, '                          functions)]', '                          (range (count functions)) functions)]', edits, 'actual-task-index-sequence')
    source = once(source, '(mapv #(.get %) futures)', '(o/join-values futures (mapv #(.get %) futures))', edits, 'actual-future-get-and-join-observation')
    return source, edits


def main(source, benchmark, variant):
    assert benchmark in {'binarytrees', 'knucleotide', 'pidigits'} and variant in {'compact', 'optimized'}
    original = source
    edits = []
    namespace = 'clbg.' + benchmark + '.' + variant
    observed_namespace = 'clbg.' + benchmark + 'observer.' + variant
    source = once(source, '(ns ' + namespace + '\n', '(ns ' + observed_namespace + '\n', edits, 'separate-observer-main-namespace')
    source = once(source, '[clbg.support :as s]', '[clbg.work.support :as s]', edits, 'separate-observer-helper-binding')
    source = once(source, '(:require ', '(:require [clbg.work.observer :as o] ', edits, 'observer-import')
    if benchmark == 'binarytrees':
        source = once(source, '(make-tree (inc maximum))',
            '(o/tree-root (make-tree (inc maximum)) (inc maximum) "stretch" -1 check-tree)', edits, 'actual-stretch-graph')
        source = once(source, 'retained (make-tree maximum)',
            'retained (o/tree-root (make-tree maximum) maximum "retained-before" -1 check-tree)', edits, 'actual-retained-before-graph')
        source = once(source, '(make-tree depth)',
            '(o/tree-root (make-tree depth) depth "temporary" index check-tree)', edits, 'every-actual-temporary-root')
        source = once(source, '(check-tree retained)',
            '(check-tree (o/tree-root retained maximum "retained-after" -1 check-tree))', edits, 'actual-retained-after-graph')
        if variant == 'compact':
            source = once(source, '            total (loop',
                '            observed-start (o/start-interval depth 0 iterations)\n            total (loop', edits, 'serial-interval-start')
            source = once(source, '        (s/printf "%d\\t trees of depth %d\\t check: %d\\n" iterations depth total)',
                '        (o/end-interval depth 0 iterations)\n        (s/printf "%d\\t trees of depth %d\\t check: %d\\n" iterations depth total)', edits, 'serial-interval-finish')
        else:
            old = '(defn interval [depth begin end]\n  (loop [index begin sum 0]\n    (if (= index end) sum\n      (recur (inc index) (+ sum (check-tree (o/tree-root (make-tree depth) depth "temporary" index check-tree)))))))'
            new = '(defn interval [depth begin end]\n  (o/start-interval depth begin end)\n  (try\n    (loop [index begin sum 0]\n      (if (= index end) sum\n        (recur (inc index) (+ sum (check-tree (o/tree-root (make-tree depth) depth "temporary" index check-tree))))))\n    (finally (o/end-interval depth begin end))))'
            source = once(source, old, new, edits, 'actual-worker-interval-start-and-finish')
    elif benchmark == 'knucleotide':
        source = once(source, '(defn histogram [^String sequence width]', '(defn histogram-raw [^String sequence width]', edits, 'observed-return-wrapper-name')
        if variant == 'compact':
            source = once(source, '(transient PersistentHashMap/EMPTY)', '(o/map-start (transient PersistentHashMap/EMPTY) sequence width)', edits, 'actual-private-transient-construction')
            old = '(assoc! table key (inc (get table key 0)))'
            source = once(source, old, '(o/map-write table key position width (fn [] ' + old + '))', edits, 'every-actual-transient-write')
            source = once(source, 'tables (mapv #(histogram sequence %) widths)',
                'tables (o/maps-received (mapv #(histogram sequence %) widths))', edits, 'actual-parent-seven-map-receipt')
        else:
            source = once(source, '(HashMap.)', '(o/map-start (HashMap.) sequence width)', edits, 'actual-private-HashMap-construction')
            old = '(.put table next-key (inc (long (.getOrDefault table next-key 0))))'
            source = once(source, old, '(o/map-write table next-key (- (inc position) width) width (fn [] ' + old + '))', edits, 'every-actual-overlapping-HashMap-write')
            source = once(source, 'tables (s/jobs (mapv (fn [width] #(histogram sequence width)) widths))',
                'tables (o/maps-received (s/jobs (mapv (fn [width] #(histogram sequence width)) widths)))', edits, 'actual-joined-seven-map-parent-receipt')
        marker = '(defn frequencies '
        wrapper = '(defn histogram [sequence width]\n  (o/map-return (histogram-raw sequence width) sequence width))\n'
        source = once(source, marker, wrapper + marker, edits, 'actual-persistent-or-HashMap-return-inventory')
    else:
        marker = '              ^BigInteger four (.divide numerator-four t)]\n'
        hook = '          (o/pi-step q r t k emitted numerator-three numerator-four three four)\n'
        source = once(source, marker, marker + hook, edits, 'both-actual-division-operands-and-quotients')
    source = once(source, '(defn -main [& arguments]', '(defn run-scored [& arguments]', edits, 'bounded-observer-main-wrapper')
    wrapper = '\n(defn -main [& arguments]\n  (try (apply run-scored arguments) (finally (o/finish))))\n'
    source += wrapper
    edits.append(dict(role='flush-actual-observation-evidence-after-original-main', original='', observed=wrapper, appended_once=True))
    if benchmark == 'binarytrees':
        # Remove the copied creator/checker. Every live graph is made/checked by
        # the unchanged, separately pinned scored classes on the classpath.
        begin = original.index('(defn make-tree ')
        end = original.index('(defn -main ' if variant == 'compact' else '(defn interval ', begin)
        source = once(source, original[begin:end], '', edits, 'use-original-compiled-creator-and-checker')
        source = once(source, '(:require ', '(:require [' + namespace + ' :as scored] ', edits, 'original-compiled-tree-namespace')
        for name in ('make-tree', 'check-tree'):
            occurrences = source.count(name)
            assert occurrences > 0 and ('(defn ' + name + ' ') not in source
            source = source.replace(name, 'scored/' + name)
            edits.append(dict(role='actual-original-compiled-' + name, original=name,
                observed='scored/' + name, expected_occurrences=occurrences))
    return source, edits
