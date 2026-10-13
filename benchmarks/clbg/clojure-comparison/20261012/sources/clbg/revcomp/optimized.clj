(ns clbg.revcomp.optimized
  (:require [clbg.support :as s] [clojure.string :as string])
  (:import [java.nio.charset StandardCharsets])
  (:gen-class))
(def complements (zipmap "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn" "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN"))
(defn records [raw]
  (reduce (fn [result ^String line]
            (cond
              (.startsWith line ">") (conj result [line (StringBuilder.)])
              (or (empty? result) (.startsWith line ";")) result
              :else (do (.append ^StringBuilder (second (peek result)) line) result)))
          [] (string/split raw #"\r\n|\r|\n" -1)))
(defn complement-byte [value]
  (let [character (char (bit-and 255 value))]
    (unchecked-byte (int (get complements character character)))))
(defn -main [& arguments]
  (doseq [[header ^StringBuilder sequence] (records (s/input-text))]
    (println header)
    (let [bytes (.getBytes (.toString sequence) StandardCharsets/ISO_8859_1)
          length (alength bytes)]
      (loop [left 0 right (dec length)]
        (when (<= left right)
          (let [a (complement-byte (aget bytes left)) b (complement-byte (aget bytes right))]
            (aset-byte bytes left b)
            (aset-byte bytes right a))
          (recur (inc left) (dec right))))
      (loop [start 0]
        (when (< start length)
          (.write System/out bytes start (min 60 (- length start)))
          (.write System/out 10)
          (recur (+ start 60)))))))

