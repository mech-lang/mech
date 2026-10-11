;;    The Computer Language Benchmarks Game
;;    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;
;;    Adapted from the C (gcc) code by Sebastien Loisel
;;    Contributed by Christopher Neufeld 2005-08-19
;;    Modified by Juho Snellman 2005-10-26
;;      * Use SIMPLE-ARRAY instead of ARRAY in declarations
;;      * Rearrange EVAL-A to make it more readable and a bit faster
;;    Modified by Andy Hefner 2008-09-18
;;      * Eliminate array consing
;;      * Clean up type declarations in eval-A
;;    Modified by Isaac Gouy 2019-10-17
;;      * eval-A like C gcc #4 program
;;      * posix-argv like Jon Smith's fannkuch-redux Lisp SBCL #2 program
;;      * deftype suggested by tfb on SO

(deftype int31 (&optional (bits 31))
  `(signed-byte ,bits))

(declaim (inline eval-A))
(defun eval-A (i j)
  (declare (type int31 i j))
  (/ 1.0d0 (+ (ash (* (+ i j) (+ i j 1)) -1) i 1)))

(defun eval-A-times-u (n u)
  (declare (type int31 n)
           (type (simple-array double-float) u))
  (let ((retval (make-array n :element-type 'double-float :initial-element 0.0d0)))
    (dotimes (i n)
      (dotimes (j n)
        (incf (aref retval i) (* (eval-A i j) (aref u j)))))
    retval))

(defun eval-At-times-u (n u)
  (declare (type int31 n)
           (type (simple-array double-float) u))
  (let ((retval (make-array n :element-type 'double-float :initial-element 0.0d0)))
    (dotimes (i n)
      (dotimes (j n)
        (incf (aref retval i) (* (eval-A j i) (aref u j)))))
    retval))

(defun eval-AtA-times-u (n u)
  (eval-At-times-u n (eval-A-times-u n u)))

(defun main () 
  (let* ((args (cdr sb-ext:*posix-argv*))
         (n (parse-integer (car args))))
    (declare (type int31 n))
    (let ((u (make-array n :element-type 'double-float :initial-element 1.0d0))
          (v (make-array n :element-type 'double-float)))
      (declare (type (simple-array double-float) U V))
      (dotimes (i 10)
        (setf v (eval-AtA-times-u n u))
        (setf u (eval-AtA-times-u n v)))
      (let ((vBv 0.0d0)
            (vv 0.0d0))
        (declare (type double-float vBv vv))
        (dotimes (i n)
          (incf vBv (* (aref u i) (aref v i)))
          (incf vv (* (aref v i) (aref v i))))
        (format t "~11,9F~%" (sqrt (the (double-float 0d0) (/ vBv vv))))))))
