(ns clbg.knucleotide.optimized
  (:require [clbg.support :as s] [clojure.string :as string])
  (:import [java.util HashMap Locale])
  (:gen-class))
(def widths [1 2 3 4 6 12 18])
(def queries ["GGT" "GGTA" "GGTATT" "GGTATTTTAATT" "GGTATTTTAATTTATAGT"])
(defn third-record [raw]
  (loop [lines (seq (string/split raw #"\r\n|\r|\n" -1)) selected false output (StringBuilder.)]
    (if-let [^String line (first lines)]
      (cond
        (.startsWith line ";") (recur (next lines) selected output)
        (.startsWith line ">") (if selected (.toUpperCase (.toString output) Locale/ROOT)
                                  (recur (next lines) (or (= line ">THREE")
                                                        (.startsWith line ">THREE ")
                                                        (.startsWith line ">THREE\t")) output))
        selected (do (.append output line) (recur (next lines) true output))
        :else (recur (next lines) false output))
      (.toUpperCase (.toString output) Locale/ROOT))))
(defn code [character]
  (case character \A 0 \C 1 \G 2 \T 3 (throw (IllegalArgumentException. "Invalid nucleotide"))))
(defn histogram [^String sequence width]
  (let [table (HashMap.) mask (dec (bit-shift-left 1 (* 2 width)))]
    (loop [position 0 key 0]
      (when (< position (.length sequence))
        (let [next-key (bit-and mask (bit-or (bit-shift-left key 2) (code (.charAt sequence (int position)))))]
          (when (>= (inc position) width)
            (.put table next-key (inc (long (.getOrDefault table next-key 0)))))
          (recur (inc position) (long next-key)))))
    table))
(defn decode [key width]
  (let [word (char-array width)]
    (loop [index (dec width) remaining key]
      (when (>= index 0)
        (aset-char word index (.charAt "ACGT" (int (bit-and remaining 3))))
        (recur (dec index) (unsigned-bit-shift-right remaining 2))))
    (String. word)))
(defn frequencies [^HashMap table width]
  (let [rows (mapv (fn [entry] [(decode (.getKey entry) width) (.getValue entry)]) (.entrySet table))
        total (reduce + 0 (map second rows))]
    (doseq [[word amount] (sort-by (fn [[word amount]] [(- amount) word]) rows)]
      (s/printf "%s %.3f\n" word (/ (* 100.0 amount) total)))
    (println)))
(defn -main [& arguments]
  (let [sequence (third-record (s/input-text))
        tables (s/jobs (mapv (fn [width] #(histogram sequence width)) widths))]
    (doseq [index (range 2)] (frequencies (nth tables index) (nth widths index)))
    (doseq [[query ^HashMap table] (map vector queries (drop 2 tables))]
      (let [key (reduce (fn [key character] (bit-or (bit-shift-left key 2) (code character))) 0 query)]
        (s/printf "%d\t%s\n" (.getOrDefault table key 0) query)))))
