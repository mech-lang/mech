;;   The Computer Language Benchmarks Game
;;   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;;
;;; resubmitted by Wade Humeniuk (Fix Stream Problem)
;;; resubmitted by Jon Smith (Remove silly assertion causing it to break on 16000 size)
;;; re-resubmitted by Jon Smith (with a silly hack to make it parallel).
;;; Original contributed by Yannick Gingras
;;;
;;; To compile
;;; sbcl --load mandelbrot.lisp --eval "(save-lisp-and-die \"mandelbrot.core\" :purify t :toplevel (lambda () (main) (quit)))"
;;; To run
;;; sbcl --noinform --core mandelbrot.core %A

(defun render (size stream)
  (declare (type fixnum size) (stream stream)
	   (optimize (speed 3) (safety 0) (debug 0)))
  (let* ((code 0) 
	 (bit 0)
         (zr 0.0d0)
	 (zi 0.0d0)
	 (tr 0.0d0)
	 (delta (/ 2d0 size))
         (base-real -1.5d0) 
	 (base-imag -1.0d0)
         (buffer (make-array (* size (ceiling size 8)) :element-type '(unsigned-byte 8)))
         (index 0))

    (declare (type (unsigned-byte 8) code )
             (type double-float zr zi tr base-real base-imag delta)
             (type fixnum index bit))

    (dotimes (y size)
      (setf base-imag (- 1.0d0 (* delta y)))
      (dotimes (x size)
	(setf base-real (+ -1.5d0 (* delta x))
	      zr base-real
	      zi base-imag)
        (setf code  
	      (if (dotimes (n 50)
		    (when (< 4.0d0 (+ (* zr zr) (* zi zi)))
		      (return t))
		    (setf tr (+ (* zr zr) (- (* zi zi)) base-real)
			  zi (+ (* 2.0d0 zr zi) base-imag)
			  zr tr))
		  (ash code 1)
		  (logior (ash code 1) #x01)))
	(when (= (incf bit) 8)
	  (setf (aref buffer index) code
		bit 0 code 0)
	  (incf index))))

    (write-sequence buffer stream)))

(defun par-render (size stream)
  (declare (type fixnum size)		(stream stream)
	   (optimize (speed 3) (safety 0) (debug 0)))

  (let* ((buffer (make-array (* size (ceiling size 8)) :element-type '(unsigned-byte 8)))
	 (quarter-size (ceiling size 4))
	 (quarter-array (ceiling (the (unsigned-byte 32) (* size size)) 32)))
    

    (labels ((render-sub (y-start y-end index)
	       (let ((code 0) 
		     (bit 0)
		     (zr 0.0d0) (zi 0.0d0) (tr 0.0d0)
		     (delta (/ 2d0 size))
		     (base-real -1.5d0)  (base-imag -1.0d0))
		 (declare (type (unsigned-byte 8) code)
			  (type double-float zr zi tr base-real base-imag delta)
			  (type fixnum index bit))


		 (do ((y y-start (1+ y)))
		     ((= y y-end))
		   (declare (type (unsigned-byte 32) y))

		   (setf base-imag (- 1.0d0 (* delta y)))
		   (dotimes (x size)
		     (setf base-real (+ -1.5d0 (* delta x))
			   zr base-real
			   zi base-imag)
		     (setf code  
			   (if (dotimes (n 50)
				 (when (< 4.0d0 (+ (* zr zr) (* zi zi)))
				   (return t))
				 (setf tr (+ (* zr zr) (- (* zi zi)) base-real)
				       zi (+ (* 2.0d0 zr zi) base-imag)
				       zr tr))
			       (ash code 1)
			       (logior (ash code 1) #x01)))
		     (when (= (incf bit) 8)
		       (setf (aref buffer index) code
			     bit 0 
			     code 0)
		       (incf index))
		     )))))
      (let (threads)
	(dotimes (i 4)
	  (let ((start (* i quarter-size))
		(end (* (+ i 1) quarter-size))
		(idx (* i quarter-array)))
	    (push (sb-thread:make-thread (lambda () (render-sub start end idx))) threads)))
	(dolist (thread threads)
	  (sb-thread:join-thread thread)))
      (write-sequence buffer stream))))

(defun main ()
  (declare (optimize (speed 0) (safety 3)))
  (let* ((args sb-ext:*posix-argv*)
	 (n (parse-integer (second args))))
    (with-open-stream (stream (sb-sys:make-fd-stream (sb-sys:fd-stream-fd sb-sys:*stdout*)
						     :element-type :default
						     :buffering :full
						     :output t :input nil))

      (format stream "P4~%~d ~d~%" n n)
      #+sb-thread(par-render n stream)
      #-sb-thread(render n stream)
      (force-output stream))))
