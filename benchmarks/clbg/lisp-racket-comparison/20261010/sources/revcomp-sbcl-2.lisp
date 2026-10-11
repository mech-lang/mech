;;; The Computer Language Benchmarks Game
;;; https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;; contributed by Antonio Saade

(declaim (optimize (speed 3) (safety 0) (debug 0))
         (inline compl))

(defconstant +lut+
  (let ((lut (make-array 256 :element-type 'base-char)))
    (loop
      for a across "ACGTMRWSYKVHDBNUacgtmrwsykvhdbnu"
      for b across "TGCAKYWSRMBDHVNATGCAKYWSRMBDHVNA"
      do (setf (aref lut (char-code a)) b))
    lut))

(defun compl (a)
  (declare (type base-char a))
  (aref +lut+ (char-code a)))

(defun write-seq-by-n (s n out)
  (declare (type simple-string s)
           (type fixnum n))
  (loop
    with length = (length s)
    for start from 0 by n below length
    do (write-line s out :start start :end (min length (+ start n)))))

(defun process-fasta (s out)
  (declare (type simple-string s))
  (do ((start 0 (1+ start))
       (end (- (length s) 1) (1- end))
       (b #\0))
      ((< end start))
    (setf b (schar s start))
    (setf (schar s start) (compl (schar s end)))
    (setf (schar s end) (compl b)))
  (write-seq-by-n s 60 out))

(defun main ()
  (with-open-file (in #p"/dev/stdin" :direction :input)
    (with-open-file (out #p"/dev/stdout"
                         :direction :output
                         :if-exists :supersede)
      (loop
        with fasta = (make-string-output-stream :element-type 'base-char)
        for line simple-string = (read-line in nil)
        while line
        do (if (char= (char line 0) #\>)
               (progn
                 (process-fasta (get-output-stream-string fasta) out)
                 (write-line line out))
               (write-string line fasta))
        finally (process-fasta (get-output-stream-string fasta) out)))))
