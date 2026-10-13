(ns clbg.pidigitsobserver.optimized
  (:require [clbg.work.observer :as o] [clbg.work.support :as s])
  (:import [java.math BigInteger])
  (:gen-class))
(def two (BigInteger/valueOf 2))
(def three-coefficient (BigInteger/valueOf 3))
(def four-coefficient (BigInteger/valueOf 4))
(def ten (BigInteger/valueOf 10))
(defn integer [value] (BigInteger/valueOf (long value)))
(defn run-scored [& arguments]
  (let [wanted (s/argument arguments 27)
        line (StringBuilder. 10)
        output (StringBuilder.)]
    (loop [^BigInteger q BigInteger/ONE ^BigInteger r BigInteger/ZERO ^BigInteger t BigInteger/ONE
           k 1 emitted 0]
      (if (= emitted wanted)
        (do (print (.toString output)) (flush))
        (let [numerator-three (.add (.multiply q three-coefficient) r)
              numerator-four (.add (.multiply q four-coefficient) r)
              ^BigInteger three (.divide numerator-three t)
              ^BigInteger four (.divide numerator-four t)]
          (o/pi-step q r t k emitted numerator-three numerator-four three four)
          (if (.equals three four)
            (let [number (inc emitted)]
              (.append line (.intValue three))
              (when (or (zero? (mod number 10)) (= number wanted))
                (.append output (String/format java.util.Locale/ROOT "%-10s\t:%d\n" (to-array [(.toString line) number])))
                (.setLength line 0))
              (recur (.multiply q ten) (.multiply (.subtract r (.multiply t three)) ten) t k number))
            (let [factor (integer (inc (* 2 k)))]
              (recur (.multiply q (integer k)) (.multiply (.add r (.multiply q two)) factor)
                     (.multiply t factor) (inc k) emitted))))))))

(defn -main [& arguments]
  (try (apply run-scored arguments) (finally (o/finish))))
