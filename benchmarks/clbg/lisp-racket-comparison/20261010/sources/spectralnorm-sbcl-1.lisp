;;   The Computer Language Benchmarks Game
;;   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;
;;    Adapted from the C (gcc) code by Sebastien Loisel
;;
;;    Contributed by Christopher Neufeld
;;    Modified by Juho Snellman 2005-10-26
;;      * Use SIMPLE-ARRAY instead of ARRAY in declarations
;;      * Use TRUNCATE instead of / for fixnum division
;;      * Rearrange EVAL-A to make it more readable and a bit faster

;; Note that sbcl is at least 10 times faster than either clisp or gcl
;; on this program, running with an argument of 500.  It would be nice
;; to know why the others are so slow.

(defun eval-AtA-times-u (n u)
  (eval-At-times-u n (eval-A-times-u n u)))

;; This is our most expensive function.  Optimized with the knowledge
;; that 'n' will never be "huge".  This will break if 'n' exceeds
;; approximately half of the square root of the largest fixnum
;; supported by the implementation.  On sbcl 0.9.3,
;; 'most-positive-fixnum' is 536870911, and we can support values of
;; 'n' above 11000.
(declaim (inline eval-A))
(defun eval-A (i j)
  (declare (type fixnum i j))
  (let* ((n (+ i j))
         (n+1 (1+ n)))
    (declare (fixnum n n+1))
    (/ (float (+ (truncate (the fixnum (* n n+1)) 2) i 1) 0d0))))

(defun eval-A-times-u (n u)
  (declare (type fixnum n)
           (type (simple-array double-float) u))
  (let ((retval (make-array n :element-type 'double-float :initial-element 0.0d0)))
    (dotimes (i n)
      (dotimes (j n)
        (incf (aref retval i) (* (eval-A i j) (aref u j)))))
    retval))

(defun eval-At-times-u (n u)
  (declare (type fixnum n)
           (type (simple-array double-float) u))
  (let ((retval (make-array n :element-type 'double-float :initial-element 0.0d0)))
    (dotimes (i n)
      (dotimes (j n)
        (incf (aref retval i) (* (eval-A j i) (aref u j)))))
    retval))


(defun main (&optional n-supplied)
  (let ((n (or n-supplied
               (parse-integer (or (car (last #+sbcl sb-ext:*posix-argv*
                                             #+clisp ext:*args*
                                             #+cmu extensions:*command-line-strings*
                                             #+gcl  si::*command-args*))
                                  "2000")))))
    (or (typep (* (- (* 2 n) 1) (- (* 2 n) 2)) 'fixnum)
        (error "The supplied value of 'n' breaks the optimizations in EVAL-A"))
    (let ((u (make-array n :element-type 'double-float :initial-element 1.0d0))
          (v (make-array n :element-type 'double-float)))
      (declare (type (simple-array double-float) U V))
      (dotimes (i 10)
        (setf v (eval-AtA-times-u n u))
        (setf u (eval-AtA-times-u n v)))
      (let ((vBv 0.0d0)
            (vv 0.0d0))
        (dotimes (i n)
          (incf vBv (* (aref u i) (aref v i)))
          (incf vv (* (aref v i) (aref v i))))
        (format t "~11,9F~%" (sqrt (the (double-float 0d0) (/ vBv vv))))))))

;; #+sbcl (progn
;;          (sb-profile:profile eval-AtA-times-u)
;;          (sb-profile:profile eval-A)
;;          (sb-profile:profile eval-A-times-u)
;;          (sb-profile:profile eval-At-times-u)
;;          (sb-profile:profile main))
