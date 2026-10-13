(ns clbg.binarytreesobserver.optimized
  (:require [clbg.binarytrees.optimized :as scored] [clbg.work.observer :as o] [clbg.work.support :as s])
  (:gen-class))
(defn interval [depth begin end]
  (o/start-interval depth begin end)
  (try
    (loop [index begin sum 0]
      (if (= index end) sum
        (recur (inc index) (+ sum (scored/check-tree (o/tree-root (scored/make-tree depth) depth "temporary" index scored/check-tree))))))
    (finally (o/end-interval depth begin end))))
(defn run-scored [& arguments]
  (let [maximum (max 6 (s/argument arguments 10))
        stretch (scored/check-tree (o/tree-root (scored/make-tree (inc maximum)) (inc maximum) "stretch" -1 scored/check-tree))
        retained (o/tree-root (scored/make-tree maximum) maximum "retained-before" -1 scored/check-tree)]
    (s/printf "stretch tree of depth %d\t check: %d\n" (inc maximum) stretch)
    (doseq [depth (range 4 (inc maximum) 2)]
      (let [iterations (bit-shift-left 1 (+ maximum 4 (- depth)))
            workers (min 8 iterations)
            totals (s/jobs (mapv (fn [worker]
                                  #(interval depth (quot (* iterations worker) workers)
                                                   (quot (* iterations (inc worker)) workers)))
                                (range workers)))]
        (s/printf "%d\t trees of depth %d\t check: %d\n" iterations depth (reduce + totals))))
    (s/printf "long lived tree of depth %d\t check: %d\n" maximum (scored/check-tree (o/tree-root retained maximum "retained-after" -1 scored/check-tree)))))


(defn -main [& arguments]
  (try (apply run-scored arguments) (finally (o/finish))))
