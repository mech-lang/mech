(ns clbg.knucleotideobserver.compact
  (:require [clbg.work.observer :as o] [clbg.work.support :as s] [clojure.string :as string])
  (:import [clojure.lang PersistentHashMap] [java.util Locale])
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
(defn histogram-raw [^String sequence width]
  (let [windows (max 0 (inc (- (.length sequence) width)))]
    (persistent!
      (loop [position 0 table (o/map-start (transient PersistentHashMap/EMPTY) sequence width)]
        (if (= position windows) table
          (let [key (.substring sequence (int position) (int (+ position width)))]
            (recur (inc position) (o/map-write table key position width (fn [] (assoc! table key (inc (get table key 0))))))))))))
(defn histogram [sequence width]
  (o/map-return (histogram-raw sequence width) sequence width))
(defn frequencies [table]
  (let [total (reduce + 0 (vals table))]
    (doseq [[word amount] (sort-by (fn [[word amount]] [(- amount) word]) table)]
      (s/printf "%s %.3f\n" word (/ (* 100.0 amount) total)))
    (println)))
(defn run-scored [& arguments]
  (let [sequence (third-record (s/input-text))
        tables (o/maps-received (mapv #(histogram sequence %) widths))]
    (doseq [table (take 2 tables)] (frequencies table))
    (doseq [[query table] (map vector queries (drop 2 tables))]
      (s/printf "%d\t%s\n" (get table query 0) query))))

(defn -main [& arguments]
  (try (apply run-scored arguments) (finally (o/finish))))
