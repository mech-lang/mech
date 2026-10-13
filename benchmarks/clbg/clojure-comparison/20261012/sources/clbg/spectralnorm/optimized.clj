(ns clbg.spectralnorm.optimized (:require [clbg.support :as s]) (:gen-class))
(defn denominator ^long [^long row ^long column]
  (let [sum (+ row column)] (+ (quot (* sum (inc sum)) 2) row 1)))
(defn interval [^doubles vector transpose ^doubles output begin end]
  (let [n (alength vector)]
    (loop [row (long begin)]
      (when (< row end)
        (aset-double output row
          (loop [column 0 sum 0.0]
            (if (= column n) sum
              (recur (inc column)
                (+ sum (/ (aget vector column)
                          (double (if transpose (denominator column row) (denominator row column)))))))))
        (recur (inc row))))))
(defn multiply [^doubles vector transpose]
  (let [n (alength vector) output (double-array n) workers (min 8 n)]
    (s/jobs (mapv (fn [worker]
                   #(interval vector transpose output (quot (* n worker) workers) (quot (* n (inc worker)) workers)))
                 (range workers)))
    output))
(defn ata [vector] (multiply (multiply vector false) true))
(defn result [^doubles u ^doubles v]
  (loop [index 0 uv 0.0 vv 0.0]
    (if (= index (alength u)) (Math/sqrt (/ uv vv))
      (recur (inc index) (+ uv (* (aget u index) (aget v index))) (+ vv (* (aget v index) (aget v index)))))))
(defn -main [& arguments]
  (let [n (int (s/argument arguments 100))]
    (loop [iteration 0 u (double-array n 1.0) v (double-array n)]
      (if (= iteration 10) (s/printf "%.9f\n" (result u v))
        (let [next-v (ata u) next-u (ata next-v)]
          (recur (inc iteration) next-u next-v))))))

