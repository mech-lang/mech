(ns clbg.revcomp.compact
  (:require [clbg.support :as s] [clojure.string :as string])
  (:gen-class))
(def complements (zipmap "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn" "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN"))
(defn records [raw]
  (reduce (fn [result ^String line]
            (cond
              (.startsWith line ">") (conj result [line (StringBuilder.)])
              (or (empty? result) (.startsWith line ";")) result
              :else (do (.append ^StringBuilder (second (peek result)) line) result)))
          [] (string/split raw #"\r\n|\r|\n" -1)))
(defn -main [& arguments]
  (doseq [[header ^StringBuilder sequence] (records (s/input-text))]
    (println header)
    (let [output (StringBuilder.)]
      (loop [index (dec (.length sequence)) column 0]
        (when (>= index 0)
          (.append output (char (get complements (.charAt sequence (int index)) (.charAt sequence (int index)))))
          (if (or (= column 59) (zero? index))
            (do (println (.toString output)) (.setLength output 0) (recur (dec index) 0))
            (recur (dec index) (inc column))))))))

