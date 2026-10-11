;;   The Computer Language Benchmarks Game
;;   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;; contributed by Ralph Richard Cook
;;; Optimizations by Nicolas Neuss
;;; Includes code from the 'random' benchmark
;;; Credits from the 'random' benchmark:
;;; Chih-Chung Chang contributed better declarations to improve performance
;;; Multi-lisp modifications by Brent Fulgham

(defconstant IM     139968)
(defconstant IA       3877)
(defconstant IC     29573)

(defparameter THE_LAST 42)

(declaim (inline gen_random select-random))

(defun gen_random (max)
  (declare (type (unsigned-byte 30) IM IA IC THE_LAST))
  (declare (double-float max))
  (setq THE_LAST (mod (+ (the (unsigned-byte 31) (* THE_LAST IA)) IC) IM))
  (/ (* max THE_LAST) IM))

(defconstant LINE_LENGTH 60)

(defstruct (freq (:type vector))
  (c #\x :type base-char)
  (p 0.0d0 :type double-float))

(defparameter ALU (concatenate 'string
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"))

(defparameter IUB 
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

(defparameter HomoSapiens 
  (vector
   (make-freq :c #\a :p 0.3029549426680d0)
   (make-freq :c #\c :p 0.1979883004921d0)
   (make-freq :c #\g :p 0.1975473066391d0)
   (make-freq :c #\t :p 0.3015094502008d0)))

(defun make-cumulative (freqs)
  (let ((cp 0.0d0))
    (declare (double-float cp))
    (dotimes (i (length freqs))
      (incf cp (freq-p (aref freqs i)))
      (setf (freq-p (aref freqs i)) cp))))


(defun select-random (freqs len)
  (declare (fixnum len) (simple-vector freqs))
  (let ((r (gen_random 1.0d0)))
    (declare (double-float r))
    (dotimes (i len)
      (when (< r (freq-p (aref freqs i)))
        (return-from select-random (freq-c (aref freqs i)))))
    (freq-c (aref freqs (1- len)))))

(defconstant BUFFER_SIZE 30000)
(defun make-random-fasta (id desc freqs freqs-length n line-length)
  (declare (fixnum freqs-length n line-length))
  (let ((buf (make-string BUFFER_SIZE :element-type 'base-char :initial-element #\Space))
        (index 0))
    (declare (fixnum index))
    (format t ">~a ~a~%" id desc)
    (do* ((chars-remaining n (- chars-remaining current-line-length))
          (current-line-length 0))
         ((<= chars-remaining 0) nil)
      (declare (fixnum chars-remaining current-line-length))
      (setf current-line-length (min chars-remaining line-length))
      (when (< (- BUFFER_SIZE index) current-line-length)
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
  (declare (fixnum n line-length))
  (let ((s-start 0)               ; this won't change
        (s-end (length s))        ; this won't change
        (w-start 0)               ; this will cycle around
        (w-end 0)                 ; this will cycle around
        (current-line-length 0))  ; line-length or less for the last line
    (declare (fixnum s-start s-end w-start w-end))
    (format t ">~a ~a~%" id desc)
    (do ((chars-remaining n (- chars-remaining current-line-length)))
        ((<= chars-remaining 0) nil)
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
  (let ((n (parse-integer (or (car (last #+sbcl sb-ext:*posix-argv*
                                         #+cmu  extensions:*command-line-strings*
                                         #+gcl  si::*command-args*)) "1"))))

    (make-cumulative IUB)
    (make-cumulative HomoSapiens)

    (make-repeat-fasta "ONE" "Homo sapiens alu" ALU (* n 2) LINE_LENGTH)
    (make-random-fasta "TWO" "IUB ambiguity codes" IUB (length IUB) (* n 3) LINE_LENGTH)
    (make-random-fasta "THREE"  "Homo sapiens frequency" HomoSapiens (length HomoSapiens) (* n 5) LINE_LENGTH)))
