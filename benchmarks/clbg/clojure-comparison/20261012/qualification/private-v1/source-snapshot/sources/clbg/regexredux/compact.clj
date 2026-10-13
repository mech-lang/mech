(ns clbg.regexredux.compact
  (:require [clbg.support :as s])
  (:import [java.util.regex Pattern])
  (:gen-class))
(def patterns ["agggtaaa|tttaccct" "[cgt]gggtaaa|tttaccc[acg]" "a[act]ggtaaa|tttacc[agt]t" "ag[act]gtaaa|tttac[agt]ct" "agg[act]taaa|ttta[agt]cct" "aggg[acg]aaa|ttt[cgt]ccct" "agggt[cgt]aa|tt[acg]accct" "agggta[cgt]a|t[acg]taccct" "agggtaa[cgt]|[acg]ttaccct"])
(def substitutions [["tHa[Nt]" "<4>"] ["aND|caN|Ha[DS]|WaS" "<3>"] ["a[NSt]|BY" "<2>"] ["<[^>]*>" "|"] ["\\|[^|][^|]*\\|" "-"]])
(defn count-matches [expression text]
  (let [matcher (.matcher (Pattern/compile expression) text)]
    (loop [total 0]
      (if (.find matcher) (recur (inc total)) total))))
(defn -main [& arguments]
  (let [raw (s/input-text)
        dna (.replaceAll (.matcher (Pattern/compile ">[^\n]*\n|\n") raw) "")]
    (doseq [pattern patterns]
      (s/printf "%s %d\n" pattern (count-matches pattern dna)))
    (let [changed (reduce (fn [text [expression replacement]]
                            (.replaceAll (.matcher (Pattern/compile expression) text) replacement))
                          dna substitutions)]
      (s/printf "\n%d\n%d\n%d\n" (count raw) (count dna) (count changed)))))

