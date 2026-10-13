(ns clbg.mandelbrot.optimized (:require [clbg.support :as s]) (:import [java.nio.charset StandardCharsets]) (:gen-class))
(defn inside? [^double cr ^double ci]
  (loop [iteration 0 zr 0.0 zi 0.0]
    (if (= iteration 50) true
      (let [real (+ (- (* zr zr) (* zi zi)) cr)
            imaginary (+ (* 2.0 zr zi) ci)]
        (if (> (+ (* real real) (* imaginary imaginary)) 4.0) false
          (recur (inc iteration) real imaginary))))))
(defn rows [^doubles real-coordinates n stride ^bytes bytes begin end]
  (loop [y (long begin)]
    (when (< y end)
      (let [ci (- (/ (* 2.0 y) n) 1.0)]
        (dotimes [column stride]
          (let [bits (loop [offset 0 value 0]
                       (if (= offset 8) value
                         (let [x (+ (* column 8) offset)]
                           (recur (inc offset) (bit-or (bit-shift-left value 1)
                             (if (and (< x n) (inside? (aget real-coordinates x) ci)) 1 0))))))]
            (aset-byte bytes (+ (* y stride) column) (unchecked-byte bits)))))
      (recur (inc y)))))
(defn -main [& arguments]
  (let [n (int (s/argument arguments 200)) stride (quot (+ n 7) 8) bytes (byte-array (* n stride))
        real-coordinates (double-array (map #(- (/ (* 2.0 %) n) 1.5) (range n))) workers (min 8 n)]
    (s/jobs (mapv (fn [worker] #(rows real-coordinates n stride bytes (quot (* n worker) workers) (quot (* n (inc worker)) workers))) (range workers)))
    (.write System/out (.getBytes (str "P4\n" n " " n "\n") StandardCharsets/US_ASCII))
    (.write System/out bytes)))
