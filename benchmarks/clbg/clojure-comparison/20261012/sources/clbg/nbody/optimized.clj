(ns clbg.nbody.optimized (:require [clbg.support :as s]) (:gen-class))
(def solar-mass (* 4.0 Math/PI Math/PI))
(def days-per-year 365.24)
(defn bodies []
  (object-array
    [(double-array [0.0 0.0 0.0 0.0 0.0 0.0 solar-mass])
     (double-array [4.84143144246472090 -1.16032004402742839 -0.103622044471123109 (* 0.00166007664274403694 days-per-year) (* 0.00769901118419740425 days-per-year) (* -0.0000690460016972063023 days-per-year) (* 0.000954791938424326609 solar-mass)])
     (double-array [8.34336671824457987 4.12479856412430479 -0.403523417114321381 (* -0.00276742510726862411 days-per-year) (* 0.00499852801234917238 days-per-year) (* 0.0000230417297573763929 days-per-year) (* 0.000285885980666130812 solar-mass)])
     (double-array [12.8943695621391310 -15.1111514016986312 -0.223307578892655734 (* 0.00296460137564761618 days-per-year) (* 0.00237847173959480950 days-per-year) (* -0.0000296589568540237556 days-per-year) (* 0.0000436624404335156298 solar-mass)])
     (double-array [15.3796971148509165 -25.9193146099879641 0.179258772950371181 (* 0.00268067772490389322 days-per-year) (* 0.00162824170038242295 days-per-year) (* -0.0000951592254519715870 days-per-year) (* 0.0000515138902046611451 solar-mass)])]))
(defn offset-momentum [^objects bodies]
  (dotimes [axis 3]
    (let [momentum (loop [index 0 total 0.0]
                     (if (= index 5) total
                       (let [^doubles body (aget bodies index)]
                         (recur (inc index) (+ total (* (aget body (+ axis 3)) (aget body 6)))))))]
      (aset-double ^doubles (aget bodies 0) (+ axis 3) (- (/ momentum solar-mass))))))
(defn energy [^objects bodies]
  (loop [i 0 total 0.0]
    (if (= i 5) total
      (let [^doubles a (aget bodies i)
            kinetic (* 0.5 (aget a 6) (+ (* (aget a 3) (aget a 3)) (* (aget a 4) (aget a 4)) (* (aget a 5) (aget a 5))))
            potential (loop [j (inc i) sum 0.0]
                        (if (= j 5) sum
                          (let [^doubles b (aget bodies j) dx (- (aget a 0) (aget b 0)) dy (- (aget a 1) (aget b 1)) dz (- (aget a 2) (aget b 2))]
                            (recur (inc j) (- sum (/ (* (aget a 6) (aget b 6)) (Math/sqrt (+ (* dx dx) (* dy dy) (* dz dz))))))))]
        (recur (inc i) (+ total kinetic potential))))))
(defn advance [^objects bodies pairs ^doubles deltas ^doubles magnitudes]
  (dotimes [pair 10]
    (let [[i j] (nth pairs pair) ^doubles a (aget bodies i) ^doubles b (aget bodies j)
          dx (- (aget a 0) (aget b 0)) dy (- (aget a 1) (aget b 1)) dz (- (aget a 2) (aget b 2))
          distance (Math/sqrt (+ (* dx dx) (* dy dy) (* dz dz)))]
      (aset-double deltas (* pair 3) dx)
      (aset-double deltas (inc (* pair 3)) dy)
      (aset-double deltas (+ (* pair 3) 2) dz)
      (aset-double magnitudes pair (/ 0.01 (* distance distance distance)))))
  (dotimes [pair 10]
    (let [[i j] (nth pairs pair) ^doubles a (aget bodies i) ^doubles b (aget bodies j)
          magnitude (aget magnitudes pair)]
      (dotimes [axis 3]
        (let [delta (aget deltas (+ (* pair 3) axis))]
          (aset-double a (+ axis 3) (- (aget a (+ axis 3)) (* delta (aget b 6) magnitude)))
          (aset-double b (+ axis 3) (+ (aget b (+ axis 3)) (* delta (aget a 6) magnitude)))))))
  (dotimes [i 5]
    (let [^doubles body (aget bodies i)]
      (dotimes [axis 3]
        (aset-double body axis (+ (aget body axis) (* 0.01 (aget body (+ axis 3)))))))))
(defn -main [& arguments]
  (let [n (s/argument arguments 1000) system (bodies)
        pairs (vec (for [i (range 5) j (range (inc i) 5)] [i j]))
        deltas (double-array 30) magnitudes (double-array 10)]
    (offset-momentum system)
    (s/printf "%.9f\n" (energy system))
    (loop [step 0]
      (when (< step n) (advance system pairs deltas magnitudes) (recur (inc step))))
    (s/printf "%.9f\n" (energy system))))
