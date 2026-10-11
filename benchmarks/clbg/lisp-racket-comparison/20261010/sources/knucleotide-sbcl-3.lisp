;;   The Computer Language Benchmarks Game
;;   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;   contributed by Vsevolod Dyomkin


(defun slice-and-dice (gene k ht)
  (declare (optimize (speed 3) (safety 0) (debug 0))
	   (type simple-base-string gene)
	   (type fixnum k))
  (clrhash ht)
;  (setf ht (make-hash-table :size (expt 2 k)))
  (if (= k 1)
      (dotimes (i (the fixnum (- (length gene) k)))
        (incf (the fixnum (gethash (schar gene i) ht 0))))
      (loop for i from 0 to (the fixnum (- (length gene) k))
         with gene-num = 0 and mask = (1- (expt 4 k)) do
           (setf gene-num (logand mask (letter-to-num gene i gene-num)))
           (unless (< i (the fixnum (1- k)))
             (incf (the fixnum (gethash gene-num ht 0))))))
  ht)

(declaim (inline letter-to-num))
(defun letter-to-num (gene i idx)
  (declare (optimize (speed 3) (safety 0) (debug 0))
           (type fixnum i))
  (ecase (schar gene i)
    (#\A (ash idx 2))
    (#\C (logxor (ash idx 2) 1))
    (#\G (logxor (ash idx 2) 2))
    (#\T (logxor (ash idx 2) 3))))

(declaim (inline gene-to-num))
(defun gene-to-num (gene)
  (let ((gene-num 0))
    (dotimes (i (length gene))
      (declare (fixnum i))
      (setf gene-num (letter-to-num gene i gene-num)))
    gene-num))

(declaim (inline num-to-gene))
(defun num-to-gene (num k)
  (declare (optimize (speed 3) (safety 0) (debug 0))
           (type fixnum num)
           (type fixnum k))
  (let ((gene (make-array k :element-type 'base-char))
        (acgt #(#\A #\C #\G #\T)))
    (dotimes (i k)
      (let ((pos (* i 2)))
        (declare (type fixnum pos))
        (setf (aref gene (- k i 1))
              (aref acgt (+ (if (logbitp pos num) 1 0)
                            (if (logbitp (1+ pos) num) 2 0))))))
    gene))

(defun discard-data-header (stream)
  (loop for line = (read-line stream nil nil)
	while (and line (string/= ">THREE" line :end2 6))))

(defun read-data (stream)
  (declare (optimize (speed 3) (safety 0) (debug 0)))
  (let ((data (make-array 0 :element-type 'base-char :adjustable t :fill-pointer 0)))
    (do ((line (read-line stream nil 'eof) (read-line stream nil 'eof)))
        ((or (eq line 'eof) (char= (schar line 0) #\>)))
      (if (not (char= (schar line 0) #\;))
          (dotimes (char-index (length line))
            (vector-push-extend (char-upcase (the base-char (schar line char-index))) data))))
    (coerce data 'simple-base-string)))

(defun print-precentages (gene k ht)
  (declare (type simple-base-string gene)
	   (type fixnum k))
  (let* ((gene-table (slice-and-dice gene k ht))
	 sorted-frequencies
	 (sum 0))
    (maphash (lambda (key v)
	       (incf sum v)
	       (push (cons key v)
                     sorted-frequencies))
	     gene-table)
    (setf sorted-frequencies 
	  (sort sorted-frequencies
		(lambda (v1 v2)
                  (cond ((> (cdr v1) (cdr v2)) t)
                        ((< (cdr v1) (cdr v2)) nil)
                        (t (< (car v1) (car v2)))))))
    (dolist (entry sorted-frequencies)
      (format t "~a ~,3F~%"
	      (if (= k 1) (car entry)
                  (num-to-gene (car entry) k))
	      (* (/ (cdr entry) (float sum)) 100.0)))
    (terpri)))

(defun print-occurence (slice gene k ht)
  (let ((gene-table (slice-and-dice gene k ht)))
    (format t "~A~C~A~%"
            (gethash (gene-to-num slice) gene-table 0)
            #\Tab
            slice)))

(defun main ()
  (declare (optimize (speed 0) (safety 1)))
  (discard-data-header *standard-input*)
  (let ((gene (read-data *standard-input*))
        (ht (make-hash-table :rehash-size 5.0)))
    (print-precentages gene 1 ht)
    (print-precentages gene 2 ht)
    (print-occurence "GGT" gene 3 ht)
    (print-occurence "GGTA" gene 4 ht)
    (print-occurence "GGTATT" gene 6 ht)
    (print-occurence "GGTATTTTAATT" gene 12 ht)
    (print-occurence "GGTATTTTAATTTATAGT" gene 18 ht)))


(defun test ()
  (with-open-file (*standard-input* "knucleotide-input.txt"
				    :direction :input)
    (with-open-file (*standard-output* "knucleotide-output.txt"
				       :direction :output
				       :if-does-not-exist :create
				       :if-exists :supersede)
      (main))))
