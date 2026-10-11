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
;;      * Distribute work across multiple cores on SBCL
;;    Modified by Isaac Gouy 2019-10-21
;;      * eval-A like C gcc #4 program
;;      * posix-argv like Jon Smith's fannkuch-redux Lisp SBCL #2 program
;;      * deftype & type function suggested by tfb on SO

(declaim (optimize (speed 3) (safety 0) (space 0)))

(deftype int31 (&optional (bits 31))
  `(signed-byte ,bits))

(declaim (inline eval-A))
(defun eval-A (i j)
  (declare (type int31 i j))
  (/ 1.0d0 (+ (ash (* (+ i j) (+ i j 1)) -1) i 1)))

(defun eval-At-times-u (u n Au start end)
  (declare (type int31 n start end)
           (type (simple-array double-float) u Au))
  (loop for i from start below end do
        (setf (aref Au i)
              (loop for j below n
                    summing (* (aref u j) (eval-A j i))
                    of-type double-float))))

(defun eval-A-times-u (u n Au start end)
  (declare (type int31 n start end)
           (type (simple-array double-float) u Au))
  (loop for i from start below end do
        (setf (aref Au i)
              (loop for j below n
                    summing (* (aref u j) (eval-A i j))
                    of-type double-float))))

#+sb-thread
(defun execute-parallel (start end function)
  (declare (type int31 start end))
  (let* ((num-threads 4))
    (declare (type function function))
    (loop with step = (truncate (- end start) num-threads)
          for index from start below end by step
          collecting (let ((start index)
                           (end (min end (+ index step))))
                       (sb-thread:make-thread
                        (lambda () (funcall function start end))))
          into threads
          finally (mapcar #'sb-thread:join-thread threads))))

#-sb-thread
(defun execute-parallel (start end function )
  (funcall function start end))

(defun eval-AtA-times-u (u AtAu v n start end)
  (execute-parallel start end
    (lambda (start end)
      (eval-A-times-u u n v start end)))
  (execute-parallel start end
    (lambda (start end)
      (eval-At-times-u v n AtAu start end))))

(defun main () 
  (let* ((args (cdr sb-ext:*posix-argv*))
         (n (parse-integer (car args))))
    (declare (type int31 n))
    (let ((u (make-array n :element-type 'double-float :initial-element 1.0d0))
          (v (make-array n :element-type 'double-float))
          (tmp (make-array n :element-type 'double-float)))
      (declare (type (simple-array double-float) U V tmp))
      (dotimes (i 10)
        (eval-AtA-times-u u v tmp n 0 n)
        (eval-AtA-times-u v u tmp n 0 n))
      (let ((vBv 0.0d0)
            (vv 0.0d0))
        (declare (type double-float vBv vv))
        (dotimes (i n)
          (incf vBv (* (aref u i) (aref v i)))
          (incf vv (* (aref v i) (aref v i))))
        (format t "~11,9F~%" (sqrt (the (double-float 0d0) (/ vBv vv))))))))
