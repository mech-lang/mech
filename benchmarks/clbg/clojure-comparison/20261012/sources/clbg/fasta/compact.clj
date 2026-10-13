(ns clbg.fasta.compact (:require [clbg.support :as s]) (:gen-class))
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
(defn repeat-record [header ^String sequence length]
  (println header)
  (loop [position 0]
    (when (< position length)
      (let [width (min 60 (- length position)) line (StringBuilder. 60)]
        (dotimes [offset width]
          (.append line (.charAt sequence (int (mod (+ position offset) (.length sequence))))))
        (println (.toString line))
        (recur (+ position width))))))
(defn random-record [header state alphabet probabilities length]
  (println header)
  (let [cumulative (double-array (rest (reductions + 0.0 probabilities)))]
    (loop [position 0]
      (when (< position length)
        (let [width (min 60 (- length position)) line (StringBuilder. 60)]
          (dotimes [_ width] (.append line (char (random-character state alphabet cumulative))))
          (println (.toString line))
          (recur (+ position width)))))))
(defn -main [& arguments]
  (let [n (s/argument arguments 27) state (int-array [42])]
    (repeat-record ">ONE Homo sapiens alu" alu (* 2 n))
    (random-record ">TWO IUB ambiguity codes" state iub-alphabet iub-probabilities (* 3 n))
    (random-record ">THREE Homo sapiens frequency" state "acgt" human-probabilities (* 5 n))))

