(ns clbg.binarytrees.compact
  (:require [clbg.support :as s])
  (:gen-class))
(defn make-tree [depth]
  (let [node (object-array 2)]
    (when (pos? depth)
      (aset node 0 (make-tree (dec depth)))
      (aset node 1 (make-tree (dec depth))))
    node))
(defn check-tree [^objects node]
  (if (nil? (aget node 0))
    1
    (+ 1 (check-tree (aget node 0)) (check-tree (aget node 1)))))
(defn -main [& arguments]
  (let [maximum (max 6 (s/argument arguments 10))
        stretch (check-tree (make-tree (inc maximum)))
        retained (make-tree maximum)]
    (s/printf "stretch tree of depth %d\t check: %d\n" (inc maximum) stretch)
    (doseq [depth (range 4 (inc maximum) 2)]
      (let [iterations (bit-shift-left 1 (+ maximum 4 (- depth)))
            total (loop [index 0 sum 0]
                    (if (= index iterations) sum
                      (recur (inc index) (+ sum (check-tree (make-tree depth))))))]
        (s/printf "%d\t trees of depth %d\t check: %d\n" iterations depth total)))
    (s/printf "long lived tree of depth %d\t check: %d\n" maximum (check-tree retained))))

