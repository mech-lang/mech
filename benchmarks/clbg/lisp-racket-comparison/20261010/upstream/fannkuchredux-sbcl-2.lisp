;;   The Computer Language Benchmarks Game
;;   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;;
;;; By Jon Smith
;;; Tranlated from OCaml implementation by ?, who translated it from
;;; the Scala implementation by Otto Bommer.
;;; 
;;; This is a single core implementation.
;;; I am sure that this program can be improved upon quite a lot.
;;; Most likely it will involve knowing how sbcl does its optimizations.
;;; As you can see, I simply used fixnums everywhere. There may be a better choice.
;;;
;;; To compile
;;; sbcl --load fannkuch.lisp --eval "(save-lisp-and-die \"fannkuch.core\" :purify t :toplevel (lambda () (main) (quit)))"
;;; To run
;;; sbcl --noinform --core fannkuch.core %A

(declaim (optimize (speed 3) (safety 0) (space 1) (debug 0)))

(defun fannkuch (n)
  (declare (type fixnum n))
  (let ((csum 0)
	(fmax 0))
    (declare (type fixnum fmax))
    (let ((perm (make-array n :element-type 'fixnum))
	  (copy (make-array n :element-type 'fixnum))
	  (num 0)) 

      (loop for i from 0 to (- n 1) do (setf (aref perm i) i))

      (labels ((do-iter (ht)
		 
		 (declare (type fixnum ht))
		 
		 (if (= ht 1)
		     (progn
		       (loop for i from 0 to (- n 1) do (setf (aref copy i) (aref perm i)))
		       (let ((c 0))
			 (declare (type fixnum c))
			 (let ((z (aref copy 0)))
			   (loop until (= z 0) do
				(progn
				  (loop for i from 0 to (ash z -1)
				     do (let ((temp (aref copy i))
					      (k (- z i)))
					  (setf (aref copy i) (aref copy k))
					  (setf (aref copy k) temp)))
				  (incf c)
				  (setf z (aref copy 0)))))
			 (setf csum (+ csum  (if (evenp num) c (- c))))
			 (when (> c fmax)
			   (setf fmax c)))
		       (incf num))
		     (loop for i from 1 to ht do
			  (progn (do-iter (- ht 1))
				 (let ((temp (aref perm 0))
				       (m (- ht 1)))
				   (loop for i from 1 to m do
					(setf (aref perm (- i 1)) (aref perm i)))
				   (setf (aref perm m) temp)))))))

	(do-iter n)))
    (format t "~s~%Pfannkuchen(~s) = ~s~%" csum n fmax)))


(defun main ()  
  (let* ((args (cdr sb-ext:*posix-argv*))
         (n (parse-integer (car args))))
    (fannkuch n)))
