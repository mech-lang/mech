(ns clbg.work.support
  (:require [clbg.work.observer :as o])
  (:refer-clojure :exclude [printf])
  (:import [java.nio.charset StandardCharsets]
           [java.util Locale]
           [java.util.concurrent Callable Executors TimeUnit]))
(defn argument [arguments default]
  (if (seq arguments) (Long/parseLong (first arguments)) default))
(defn input-text []
  (String. (.readAllBytes System/in) StandardCharsets/ISO_8859_1))
(defn printf [format-string & values]
  (print (String/format Locale/ROOT format-string (to-array values)))
  (flush))
(defn jobs [functions]
  (let [pool (Executors/newFixedThreadPool (min 8 (max 1 (count functions))))]
    (try
      (let [futures (mapv (fn [worker function]
                            (.submit pool ^Callable (reify Callable (call [_] (o/with-worker worker function)))))
                          (range (count functions)) functions)]
        (o/join-values futures (mapv #(.get %) futures)))
      (finally
        (.shutdown pool)
        (when-not (.awaitTermination pool 1 TimeUnit/DAYS)
          (throw (IllegalStateException. "Worker pool did not finish")))))))
