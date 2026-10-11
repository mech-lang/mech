;;   The Computer Language Benchmarks Game
;;   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;; Contributed by Gustavo Henrique Milaré

;;; This file was fetched from https://salsa.debian.org/benchmarksgame-team/benchmarksgame/ and modified.
;;; These are the original creators:

;;; contributed by Ralph Richard Cook
;;; Optimizations by Nicolas Neuss
;;; Includes code from the 'random' benchmark
;;; Credits from the 'random' benchmark:
;;; Chih-Chung Chang contributed better declarations to improve performance
;;; Multi-lisp modifications by Brent Fulgham

(defconstant +im+     139968)
(defconstant +ia+       3877)
(defconstant +ic+     29573)

(defparameter *last-random* 42)

(declaim (inline gen-random))

(defun gen-random (max)
  (declare (type (unsigned-byte 30) *last-random*))
  (declare (double-float max))
  (setq *last-random* (mod (+ (the (unsigned-byte 31)
				(* *last-random* +ia+))
			      +ic+)
			   +im+))
  (/ (* max *last-random*) +im+))

(defconstant +line-length+ 60)

(defstruct (freq (:predicate freqp))
  (c #\x :type base-char)
  (p 0.0d0 :type double-float))

(declaim (base-string *alu*))

(defparameter *alu* (concatenate 'base-string
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"))

(declaim (simple-vector *iub* *homo-sapiens*))

(defparameter *iub*
  (vector
   (make-freq :c #\a :p 0.27d0)
   (make-freq :c #\c :p 0.12d0)
   (make-freq :c #\g :p 0.12d0)
   (make-freq :c #\t :p 0.27d0)

   (make-freq :c #\B :p 0.02d0)
   (make-freq :c #\D :p 0.02d0)
   (make-freq :c #\H :p 0.02d0)
   (make-freq :c #\K :p 0.02d0)
   (make-freq :c #\M :p 0.02d0)
   (make-freq :c #\N :p 0.02d0)
   (make-freq :c #\R :p 0.02d0)
   (make-freq :c #\S :p 0.02d0)
   (make-freq :c #\V :p 0.02d0)
   (make-freq :c #\W :p 0.02d0)
   (make-freq :c #\Y :p 0.02d0)))

(defparameter *homo-sapiens*
  (vector
   (make-freq :c #\a :p 0.3029549426680d0)
   (make-freq :c #\c :p 0.1979883004921d0)
   (make-freq :c #\g :p 0.1975473066391d0)
   (make-freq :c #\t :p 0.3015094502008d0)))

(defun make-cumulative (freqs)
  (let ((cp 0.0d0))
    (declare (double-float cp)
	     (simple-vector freqs))
    (dotimes (i (length freqs))
      (incf cp (freq-p (aref freqs i)))
      (setf (freq-p (aref freqs i)) cp))))

(declaim (inline select-random))

#+nil
(defun select-random (freqs len)
  (declare (fixnum len) (simple-vector freqs))
  (let ((r (gen-random 1.0d0)))
    (declare (double-float r))
    (dotimes (i len)
      (when (< r (freq-p (aref freqs i)))
        (return-from select-random (freq-c (aref freqs i)))))
    (freq-c (aref freqs (1- len)))))

(defun select-random (freqs len)
  (declare (fixnum len) (simple-vector freqs))
  (let ((r (gen-random 1.0d0))
	(first (aref freqs 0)))
    (declare (double-float r))
    (if (< r (freq-p first))
	(freq-c first)
	(do* ((lo 0)
	      (hi (1- len))
	      (mid (truncate (+ lo hi) 2) (truncate (+ lo hi) 2))
	      (freq (aref freqs mid) (aref freqs mid)))
	     ((>= lo mid) (freq-c (aref freqs hi)))
	  (declare (fixnum lo hi mid))
	  (if (< r (freq-p freq))
	      (setf hi mid)
	      (setf lo mid))))))

(defconstant +buffer-size+ 30000)

(defun make-random-fasta (id desc freqs freqs-length n line-length)
  (declare (fixnum freqs-length n line-length)
	   (string id desc))
  (let ((buf (make-string +buffer-size+ :element-type 'base-char :initial-element #\Space))
        (index 0))
    (declare (fixnum index)
	     (base-string buf))
    (format t ">~a ~a~%" id desc)
    (do* ((chars-remaining n (- chars-remaining current-line-length))
          (current-line-length 0))
         ((<= chars-remaining 0) nil)
      (declare (fixnum chars-remaining current-line-length))
      (setf current-line-length (min chars-remaining line-length))
      (when (< (- +buffer-size+ index) current-line-length)
        (write-sequence buf *standard-output* :end index)
        (setf index 0))
      (dotimes (i current-line-length)
        (declare (fixnum i))
        (setf (aref buf index) (select-random freqs freqs-length))
        (incf index))
      (setf (aref buf index) #\Newline)
      (incf index))
    (when (> index 0)
      (write-sequence buf *standard-output* :end index))))


(defun make-repeat-fasta (id desc s n line-length)
  (declare (fixnum n line-length)
	   (string id desc s))
  (let ((s-start 0)               ; this won't change
        (s-end (length s))        ; this won't change
        (w-start 0)               ; this will cycle around
        (w-end 0)                 ; this will cycle around
        (current-line-length 0))  ; line-length or less for the last line
    (declare (fixnum s-start s-end w-start w-end current-line-length))
    (format t ">~a ~a~%" id desc)
    (do ((chars-remaining n (- chars-remaining current-line-length)))
        ((<= chars-remaining 0) nil)
      (declare (fixnum chars-remaining))
      (setf current-line-length (min chars-remaining line-length))
      (setf w-end (+ w-start current-line-length))
      (when (> w-end s-end)
	(decf w-end s-end))
      (if (> w-end w-start)
          (write-sequence s *standard-output* :start w-start :end w-end)
	(progn
	  (write-sequence s *standard-output* :start w-start :end s-end)
	  (write-sequence s *standard-output* :start s-start :end w-end)))
      (write-char #\Newline)
      (setf w-start w-end))))


(defun main ()
  (let ((n (or (ignore-errors
		 (parse-integer (car (last #+sbcl sb-ext:*posix-argv*
					   #+cmu  extensions:*command-line-strings*
					   #+gcl  si::*command-args*
					   #+clisp nil))))
	       1000)))
    (declare (fixnum n))

    (make-cumulative *iub*)
    (make-cumulative *homo-sapiens*)

    (make-repeat-fasta "ONE" "Homo sapiens alu"
		       *alu* (the fixnum (* n 2)) +line-length+)
    (make-random-fasta "TWO" "IUB ambiguity codes"
		       *iub* (length *iub*) (the fixnum (* n 3)) +line-length+)
    (make-random-fasta "THREE"  "Homo sapiens frequency"
		       *homo-sapiens* (length *homo-sapiens*)
		       (the fixnum (* n 5)) +line-length+)))
