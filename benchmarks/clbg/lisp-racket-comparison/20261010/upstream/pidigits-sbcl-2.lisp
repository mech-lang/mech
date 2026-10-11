;; The Computer Language Benchmarks Game
;; https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;
;; adapted from the Java and Python versions by Robert Brown 2006-10-06

(defconstant +digits-per-line+ 10)
(defconstant +default-stop-digits+ "1000")

(defun make-digit-generator ()
  (declare (optimize (speed 3) (debug 0) (space 0) (safety 0)))
  (let ((zq 1)
        (zr 0)
        (zt 1)
        (k 0)
        (4k+2 2)
        (2k+1 1))
    (declare (type integer zq zr zt)
             (type fixnum k 4k+2 2k+1))
    (labels ((extract (j)
               (the (integer 0 9) (floor (+ (* zq j) zr) zt)))
             (compose (aq ar at bq br bt)
               (setq zq (* aq bq)
                     zr (+ (* aq br) (* ar bt))
                     zt (* at bt))))
      #'(lambda ()
          (let ((y (extract 3)))
            (declare (type (integer 0 9) y))
            (loop while (not (= y (extract 4)))
                  do (compose zq zr zt (incf k) (incf 4k+2 4) (incf 2k+1 2))
                     (setf y (extract 3)))
            (compose 10 (* -10 y) 1 zq zr zt)
            y)))))

(defun spigot (digits)
  (declare (type fixnum digits))
  (let ((digits-printed 0)
        (next-digit (make-digit-generator)))
    (loop while (plusp digits)
          do (if (>= digits +digits-per-line+)
                 (progn (loop repeat +digits-per-line+
                              do (format t "~d" (funcall next-digit)))
                        (incf digits-printed +digits-per-line+))
                 (progn (loop repeat digits
                              do (format t "~d" (funcall next-digit)))
                        (loop repeat (- +digits-per-line+ digits)
                              do (format t " "))
                        (incf digits-printed digits)))
             (format t "~a:~d~%" #\Tab digits-printed)
             (decf digits +digits-per-line+))))


(defun main (&optional
	     (n (parse-integer
                 (or (car (last #+sbcl sb-ext:*posix-argv*
                                #+cmu extensions:*command-line-strings*
                                #+gcl si::*command-args*))
                     +default-stop-digits+))))
    (spigot n))
