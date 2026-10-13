(ns clbg.binarytrees.optimized
  (:require [clbg.support :as s])
  (:gen-class))
(defn make-tree [depth]
  (let [node (object-array 2)]
    (when (pos? depth)
      (aset node 0 (make-tree (dec depth)))
      (aset node 1 (make-tree (dec depth))))
    node))
(defn check-tree [^objects node]
  (if (nil? (aget node 0)) 1
    (+ 1 (check-tree (aget node 0)) (check-tree (aget node 1)))))
(defn interval [depth begin end]
  (loop [index begin sum 0]
    (if (= index end) sum
      (recur (inc index) (+ sum (check-tree (make-tree depth)))))))
(defn -main [& arguments]
  (let [maximum (max 6 (s/argument arguments 10))
        stretch (check-tree (make-tree (inc maximum)))
        retained (make-tree maximum)]
    (s/printf "stretch tree of depth %d\t check: %d\n" (inc maximum) stretch)
    (doseq [depth (range 4 (inc maximum) 2)]
      (let [iterations (bit-shift-left 1 (+ maximum 4 (- depth)))
            workers (min 8 iterations)
            totals (s/jobs (mapv (fn [worker]
                                  #(interval depth (quot (* iterations worker) workers)
                                                   (quot (* iterations (inc worker)) workers)))
                                (range workers)))]
        (s/printf "%d\t trees of depth %d\t check: %d\n" iterations depth (reduce + totals))))
    (s/printf "long lived tree of depth %d\t check: %d\n" maximum (check-tree retained))))

