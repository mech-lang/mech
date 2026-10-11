;; The Computer Language Benchmarks Game
;; https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;; 
;; Contributed by Karel Belohlavek
;; 
;; Single core implementation.
;; Inspired by the Erlang one by Alkis Gotovos and Maria Christakis.

(eval-when (:execute) (proclaim '(optimize (speed))))

(defun fannkuch (n)
   (labels
    ((mkperm (n)
     (let*
      ((params (loop for i from 1 to n collect (gensym)))
       (flip-branches
         (loop
          for j from 2 to n
          collect
            `((,j)
              ,(append '(flip (the fixnum (1+ acc)))
                       (reverse (subseq params 0 j))
                       (subseq params j)))))
       (nextperm-branches
         (loop
          for j from 0 below n
          for rparams = (append (subseq params 1 (1+ j))
                                (list (car params))
                                (subseq params (1+ j)))
          collect
            `((,j)
              (if (< (aref c i) i) 
               (progn
                (incf (aref c i))
                (let ((nflips (the fixnum (flip 0 ,@rparams))))
                  (declare (type fixnum nflips))
		  (cond (even (incf csum nflips)
		              (setf even nil))
		        (t (decf csum nflips)
			   (setf even t)))
                  (when (> nflips max-flips)
                   (setf max-flips nflips)))
                (perm 1 ,@rparams))
               (progn
                (setf (aref c i) 0)
                (perm (+ i 1) ,@rparams)))))))
      `(lambda ()
	(let ((csum 0)
	      (max-flips 0)
	      (even nil)
	      (c (make-array ,n :initial-element 0 :element-type 'fixnum)))
	  (declare (type fixnum csum max-flips)
	           (type boolean even)
	           (type (simple-array fixnum (,n)) c))
	  (labels
	   ((flip (acc ,@params)
             (declare (type fixnum acc ,@params))
             (ecase ,(car params) ((1) acc) ,@flip-branches))
            (perm (i ,@params)
	     (declare (type fixnum i ,@params))
	     (ecase i
	       (,n (format t "~s~%Pfannkuchen(~s) = ~s~%" csum ,n max-flips))
	       ,@nextperm-branches)))
	   (apply #'perm
	    1
	    (loop for j of-type fixnum from 1 to ,n collect j))))))))
    (funcall (compile nil (mkperm n)))))

(defun main ()  
  (let* ((args (cdr sb-ext:*posix-argv*))
         (n (parse-integer (car args))))
    (fannkuch n)))
