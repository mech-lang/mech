(ns clbg.fannkuchredux.optimized (:require [clbg.support :as s]) (:gen-class))
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
(defn carry [^ints permutation ^ints counts n]
  (loop [index 1]
    (when (< index n)
      (rotate-prefix! permutation index)
      (aset-int counts index (dec (aget counts index)))
      (when (zero? (aget counts index))
        (aset-int counts index (inc index))
        (recur (inc index))))))
(defn initial-state [n rank ^longs factorial]
  (let [permutation (int-array (range n)) counts (int-array n)]
    (loop [index (dec n) remaining rank]
      (when (pos? index)
        (let [digit (quot remaining (aget factorial index))]
          (dotimes [_ digit] (rotate-prefix! permutation index))
          (aset-int counts index (- (inc index) digit))
          (recur (dec index) (rem remaining (aget factorial index))))))
    [permutation counts]))
(defn interval [n begin end factorial]
  (let [[permutation counts] (initial-state n begin factorial)]
    (loop [rank (long begin) checksum 0 maximum 0]
      (if (= rank end) [checksum maximum]
        (let [amount (flips permutation)]
          (when (< (inc rank) end) (carry permutation counts n))
          (recur (inc rank) (long (+ checksum (if (even? rank) amount (- amount)))) (long (max maximum amount))))))))
(defn -main [& arguments]
  (let [n (int (s/argument arguments 7)) factorial (long-array (inc n))]
    (assert (pos? n))
    (aset-long factorial 0 1)
    (dotimes [index n] (aset-long factorial (inc index) (* (inc index) (aget factorial index))))
    (let [total (aget factorial n) workers (min 8 total)
          results (s/jobs (mapv (fn [worker] #(interval n (quot (* total worker) workers) (quot (* total (inc worker)) workers) factorial)) (range workers)))]
      (s/printf "%d\nPfannkuchen(%d) = %d\n" (reduce + (map first results)) n (reduce max (map second results))))))
