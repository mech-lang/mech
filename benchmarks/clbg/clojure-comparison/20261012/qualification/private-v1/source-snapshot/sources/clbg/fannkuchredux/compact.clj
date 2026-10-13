(ns clbg.fannkuchredux.compact (:require [clbg.support :as s]) (:gen-class))
(defn rotate-prefix! [^ints permutation last-index]
  (let [first-value (aget permutation 0)]
    (dotimes [index last-index] (aset-int permutation index (aget permutation (inc index))))
    (aset-int permutation last-index first-value)))
(defn flips ^long [^ints permutation]
  (let [work (aclone permutation)]
    (loop [total 0]
      (let [last-index (aget work 0)]
        (if (zero? last-index) total
          (do
            (loop [left 0 right (long last-index)]
              (when (< left right)
                (let [value (aget work left)]
                  (aset-int work left (aget work right)) (aset-int work right value))
                (recur (inc left) (dec right))))
            (recur (inc total))))))))
(defn carry [^ints permutation ^ints counts r n]
  (loop [index r]
    (if (= index n) n
      (do
        (rotate-prefix! permutation index)
        (aset-int counts index (dec (aget counts index)))
        (if (pos? (aget counts index)) index (recur (inc index)))))))
(defn -main [& arguments]
  (let [n (int (s/argument arguments 7)) permutation (int-array (range n)) counts (int-array n)]
    (assert (pos? n))
    (loop [r n rank 0 checksum 0 maximum 0]
      (let [ready (loop [index r]
                    (if (= index 1) 1
                      (do (aset-int counts (dec index) index) (recur (int (dec index))))))
            amount (flips permutation)
            next-checksum (+ checksum (if (even? rank) amount (- amount)))
            next-maximum (max maximum amount)
            next-r (carry permutation counts ready n)]
        (if (= next-r n)
          (s/printf "%d\nPfannkuchen(%d) = %d\n" next-checksum n next-maximum)
          (recur (int next-r) (inc rank) (long next-checksum) (long next-maximum)))))))
