(ns clbg.mandelbrot.compact (:require [clbg.support :as s]) (:import [java.nio.charset StandardCharsets]) (:gen-class))
(defn inside? [^double cr ^double ci]
  (loop [iteration 0 zr 0.0 zi 0.0]
    (if (= iteration 50) true
      (let [real (+ (- (* zr zr) (* zi zi)) cr)
            imaginary (+ (* 2.0 zr zi) ci)]
        (if (> (+ (* real real) (* imaginary imaginary)) 4.0) false
          (recur (inc iteration) real imaginary))))))
(defn -main [& arguments]
  (let [n (int (s/argument arguments 200)) stride (quot (+ n 7) 8) bytes (byte-array (* n stride))]
    (dotimes [y n]
      (dotimes [column stride]
        (let [bits (loop [offset 0 value 0]
                     (if (= offset 8) value
                       (let [x (+ (* column 8) offset)]
                         (recur (inc offset) (bit-or (bit-shift-left value 1)
                           (if (and (< x n) (inside? (- (/ (* 2.0 x) n) 1.5) (- (/ (* 2.0 y) n) 1.0))) 1 0))))))]
          (aset-byte bytes (+ (* y stride) column) (unchecked-byte bits)))))
    (.write System/out (.getBytes (str "P4\n" n " " n "\n") StandardCharsets/US_ASCII))
    (.write System/out bytes)))
