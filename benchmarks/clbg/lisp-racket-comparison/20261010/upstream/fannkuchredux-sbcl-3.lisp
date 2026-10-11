;;   The Computer Language Benchmarks Game
;;   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;;
;;; By Jon Smith (rev 3) added some more declarations.
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

;(setf *efficiency-note-cost-threshold* 1)
;(setf *efficiency-note-limit* 8)

(declaim (optimize (speed 3) (safety 0) (space 1) (debug 0)))

(defmacro sb (a) `(the fixnum ,a))
(deftype sb () 'fixnum)

(defmacro setlambda(n)
  (declare (type fixnum n))
  (let ((copy (gensym))
	(perm (gensym)))
  `(lambda (,perm ,copy)
     (declare (optimize (speed 3) (safety 0) (space 0) (debug 0))
      (type (simple-array sb (,n)) ,copy ,perm))
     ,@(loop for i of-type fixnum from 0 below n collect
	    `(setf (aref ,copy ,i) (aref ,perm ,i))))))

(defmacro countfliplambda (n)
  (declare (type fixnum n))
  (let ((copy (gensym))
	(c (gensym))
	(z (gensym)))
    `(lambda (,copy &aux (,c 0))
       (declare (optimize (speed 3) (safety 0) (space 0) (debug 0))
		(type sb ,c)
		(type (simple-array sb (,n)) ,copy))
       (let ((,z (aref ,copy 0)))
	 (loop until (= ,z 0) do
	      (progn
		(case ,z 
		  ,@(loop for i of-type sb from 1 to (- n 1) collect
			 `(,i
			   ,@(loop for j of-type sb from 0 to (ash i -1)
				if (not (= j (- i j)))
				collect `(rotatef (aref ,copy ,j) 
						  (aref ,copy ,(- i j)))))))
		(incf ,c)
		(setf ,z (aref ,copy 0)))))
       ,c)))

(defun fannkuch (n)
  (declare (type sb n))
  (let ((csum 0)
	(fmax 0))
    (declare (type sb csum fmax))
    (let ((perm (make-array n :element-type 'fixnum))
	  (copy (make-array n :element-type 'fixnum))
	  (num 0)
	  (cflip (the (function ((simple-array sb (*)))) (eval `(countfliplambda ,n))))
	  (copyfn (the (function ((simple-array sb (*)) (simple-array sb (*)))) (eval `(setlambda ,n)))))
      (declare (type (simple-array sb (*)) perm copy)
	       (type sb num))

      (loop for i from 0 to (- n 1) do (setf (aref perm i) i))
      
      (labels ((do-iter (ht)
		 (declare (type sb ht))
		 (if (= ht 1)
		     (progn
		       (funcall copyfn perm copy)
		       (let ((c (funcall cflip copy)))
			 (declare (type sb c))
			 (setf csum (sb (+ csum  (sb (if (evenp num) c (- c))))))
			 (when (> c fmax)
			   (setf fmax c)))
		       (incf num))
		     (loop for i of-type sb from 1 to ht do
			  (let ((m (- ht 1))) 
			    (do-iter m)
			    (let ((temp (aref perm 0)))
			      (loop for i from 1 to m do
				   (setf (aref perm (- i 1)) (aref perm i)))
			      (setf (aref perm m) temp)))))))

	(do-iter n)))
    (format t "~s~%Pfannkuchen(~s) = ~s~%" csum n fmax)))


(defun main ()  
  (let* ((args (cdr sb-ext:*posix-argv*))
         (n (parse-integer (car args))))
    (fannkuch n)))
