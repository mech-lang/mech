(ns clbg.pidigitsobserver.compact
  (:require [clbg.work.observer :as o] [clbg.work.support :as s])
  (:import [java.math BigInteger])
  (:gen-class))
(defn integer [value] (BigInteger/valueOf (long value)))
(defn run-scored [& arguments]
  (let [wanted (s/argument arguments 27)]
    (loop [^BigInteger q BigInteger/ONE ^BigInteger r BigInteger/ZERO ^BigInteger t BigInteger/ONE
           k 1 emitted 0 line ""]
      (when (< emitted wanted)
        (let [numerator-three (.add (.multiply q (integer 3)) r)
              numerator-four (.add (.multiply q (integer 4)) r)
              ^BigInteger three (.divide numerator-three t)
              ^BigInteger four (.divide numerator-four t)]
          (o/pi-step q r t k emitted numerator-three numerator-four three four)
          (if (.equals three four)
            (let [number (inc emitted)
                  text (str line (.intValue three))]
              (when (or (zero? (mod number 10)) (= number wanted))
                (s/printf "%-10s\t:%d\n" text number))
              (recur (.multiply q (integer 10))
                     (.multiply (.subtract r (.multiply t three)) (integer 10)) t k number
                     (if (zero? (mod number 10)) "" text)))
            (let [factor (integer (inc (* 2 k)))]
              (recur (.multiply q (integer k))
                     (.multiply (.add r (.multiply q (integer 2))) factor)
                     (.multiply t factor) (inc k) emitted line))))))))


(defn -main [& arguments]
  (try (apply run-scored arguments) (finally (o/finish))))
