(ns clbg.fasta.optimized (:require [clbg.support :as s]) (:import [java.io BufferedOutputStream] [java.nio.charset StandardCharsets]) (:gen-class))
(def alu "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA")
(def iub-alphabet "acgtBDHKMNRSVWY")
(def iub-probabilities [0.27 0.12 0.12 0.27 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02 0.02])
(def human-probabilities [0.3029549426680 0.1979883004921 0.1975473066391 0.3015094502008])
(defn draw ^double [^ints state]
  (let [value (int (mod (+ (* (aget state 0) 3877) 29573) 139968))]
    (aset-int state 0 value)
    (/ value 139968.0)))
(defn random-character [^ints state ^String alphabet ^doubles cumulative]
  (let [value (draw state)]
    (loop [index 0]
      (if (< value (aget cumulative index)) (.charAt alphabet index) (recur (inc index))))))
(defn header [^BufferedOutputStream output text]
  (.write output (.getBytes (str text "\n") StandardCharsets/US_ASCII)))
(defn repeat-record [^BufferedOutputStream output ^String sequence length]
  (loop [position 0]
    (when (< position length)
      (.write output (int (.charAt sequence (int (mod position (.length sequence))))))
      (when (= (mod position 60) 59) (.write output 10))
      (recur (inc position))))
  (when (pos? (mod length 60)) (.write output 10)))
(defn random-record [^BufferedOutputStream output state alphabet probabilities length]
  (let [cumulative (double-array (rest (reductions + 0.0 probabilities)))]
    (loop [position 0]
      (when (< position length)
        (.write output (int (random-character state alphabet cumulative)))
        (when (= (mod position 60) 59) (.write output 10))
        (recur (inc position))))
    (when (pos? (mod length 60)) (.write output 10))))
(defn -main [& arguments]
  (let [n (s/argument arguments 27) state (int-array [42]) output (BufferedOutputStream. System/out 65536)]
    (header output ">ONE Homo sapiens alu")
    (repeat-record output alu (* 2 n))
    (header output ">TWO IUB ambiguity codes")
    (random-record output state iub-alphabet iub-probabilities (* 3 n))
    (header output ">THREE Homo sapiens frequency")
    (random-record output state "acgt" human-probabilities (* 5 n))
    (.flush output)))
