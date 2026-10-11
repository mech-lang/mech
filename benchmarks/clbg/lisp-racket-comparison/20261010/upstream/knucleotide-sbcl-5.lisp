;; The Computer Language Benchmarks Game
;;   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;
;;   contributed by Alexey Voznyuk
;;;
;;; modified by Marko Kocic 
;;;   * add optimization declarations

(defpackage #:k-nucleotide
  (:use :cl))

(in-package :k-nucleotide)

(declaim (optimize (speed 3)(safety 0)(space 0)(debug 0)))

(defmacro with-packed-sequences ((&rest sequences) &body body)
  (loop :for (bind update length) :in sequences
     :collect `(,bind 0) :into binds
     :collect `(type (integer 0 ,(1- (expt 4 length))) ,bind) :into decls
     :collect `(,update (char) `(setf ,',bind
                                      (logior (ash ,',bind -2)
                                              (ash (logand (char-code ,char) #x6)
                                                   ,',(1- (* (1- length) 2)))))) :into updates
     :finally (return `(let (,@binds) (declare ,@decls) (macrolet (,@updates) ,@body)))))

(defmacro pack-sequence (sequence)
  `(with-packed-sequences ((bind update ,(length sequence)))
     (loop :for char :across ,sequence
        :do (update char))
     bind))

(defun unpack-sequence (length packed-seq)
  (declare (optimize (speed 3) (safety 0) (debug 0))
           (type fixnum length packed-seq))
  (with-output-to-string (seq-out)
    (loop :repeat length
       :do (write-char (ecase (logand packed-seq #x3)
                         (0 #\A) (1 #\C) (2 #\T) (3 #\G))
                       seq-out)
       :do (setf packed-seq (ash packed-seq -2)))))

(defmacro with-packed-caches-fill ((hash-access) &rest updaters)
  `(progn ,@(loop
               :for tick :from 1 :to (apply #'max (mapcar #'third updaters))
               :collect `(with-current-char (char :skip-newline t)
                           ,@(loop :for (bind update length) :in updaters
                                :collect `(,update char)
                                :when (>= tick length)
                                :collect `(,hash-access ,length ,bind))))))

(defmacro with-reading-stream ((stream &key (block-size 8192)) &body body)
  `(block outer-tag
     (let ((advance (let ((buffer (make-array ,block-size :element-type 'standard-char :initial-element #\Newline))
                          (index 0)
                          (amount 0))
                      (declare (type fixnum index amount))
                      (lambda ()
                        (prog2 (when (>= index amount)
                                 (setf amount (read-sequence buffer ,stream)
                                       index 0)
                                 (when (zerop amount)
                                   (return-from outer-tag nil)))
                            (elt buffer index)
                          (incf index))))))
       (flet ((get-char () (funcall advance)))
         (macrolet ((with-current-char ((char &key skip-newline) &body body)
                      `(let ((,char ,(if skip-newline
                                         `(loop :for ,char = (get-char) :while (char= ,char #\Newline)
                                             :finally (return ,char))
                                         `(get-char))))
                         (declare (type standard-char ,char))
                         ,@body)))
           ,@body)))))

(defmacro skip-buffer-to (&rest patterns)
  `(progn ,@(loop :for pattern :in patterns
               :collect `(loop :until (and ,@(loop :for char :across (string pattern)
                                                :collect `(with-current-char (char)
                                                            (char= char ,char))))))))

(defmacro with-dna-analyzed ((stream hash-access &key (block-size 8192)) &rest sequence-lengths)
  (loop :for length :in sequence-lengths
     :collect (gensym) :into binds
     :collect (gensym) :into updaters
     :finally (let ((desc (mapcar #'list binds updaters sequence-lengths)))
                (return `(with-packed-sequences (,@desc)
                           (with-reading-stream (,stream :block-size ,block-size)
                             (skip-buffer-to ">THREE" #\Newline)
                             (with-packed-caches-fill (,hash-access)
                               ,@desc)
                             (loop (with-current-char (char :skip-newline t)
                                     ,@(loop
                                          :for update :in updaters
                                          :for bind :in binds
                                          :for length :in sequence-lengths
                                          :collect `(,update char)
                                          :collect `(,hash-access ,length ,bind))))))))))

(defun seq= (seq-a seq-b)
  (declare (optimize (speed 3) (safety 0) (debug 0)) (type fixnum seq-a seq-b))
  (= seq-a seq-b))

(defun seq-hash (seq)
  (declare (optimize (speed 3) (safety 0) (debug 0)) (type fixnum seq))
  seq)

(sb-ext:define-hash-table-test seq= seq-hash)

(defmacro with-smart-dna-hash ((hash-access hash-loop &key (vector-threshold 1048576)) (&rest sequence-lengths) &body body)
  (loop :for length :in sequence-lengths
     :for bind = (gensym)
     :for area = (expt 4 length)
     :for vec-p = (<= area vector-threshold)
     :collect `(,bind ,(if vec-p
                           `(make-array ,area :element-type 'fixnum :initial-element 0)
                           `(make-hash-table :test ',(if (< area most-positive-fixnum) 'seq= 'eql)
                                             :rehash-size ,(expt 2 (1- length))
                                             :rehash-threshold 0.7))) :into binds
     :collect `(,length ,(if vec-p ``(elt ,',bind ,key) ``(the fixnum (gethash ,key ,',bind 0)))) :into accesses
     :collect `(,length ,(if vec-p
                             ``(loop :for i :from 0 :below ,',(expt 4 length)
                                  :for ,value = (elt ,',bind i)
                                  :for ,key = (unpack-sequence ,',length i)
                                  :unless (zerop ,value)
                                  ,@loop-keywords)
                             ``(loop :for packed-key :being :the :hash-keys :in ,',bind
                                  :for ,key = (unpack-sequence ,',length packed-key)
                                  :for ,value = (,',hash-access ,',length packed-key)
                                  ,@loop-keywords))) :into loops
     :finally (return `(let (,@binds)
                         (macrolet ((,hash-access (seq-length key) (ecase seq-length ,@accesses))
                                    (,hash-loop ((seq-length key value) &rest loop-keywords) (ecase seq-length ,@loops)))
                           ,@body)))))

(defmacro with-percentage ((hash-loop &rest seq-descriptions) &body body)
  (if (null seq-descriptions)
      `(progn ,@body)
      (destructuring-bind (seq-bind seq-length)
          (car seq-descriptions)
        `(let ((,seq-bind (,hash-loop (,seq-length k v)
                                      :summing v :into total :of-type fixnum
                                      :and :collect k :into seqs
                                      :and :collect v :into counts
                                      :finally (return (mapcar #'list
                                                               seqs
                                                               (mapcar (lambda (count)
                                                                         (declare (type fixnum count))
                                                                         (/ (* count 100.0) total))
                                                                       counts))))))
           (with-percentage (,hash-loop ,@(cdr seq-descriptions)) ,@body)))))

(defmacro obtain-seq-count (hash-access seq)
  `(list (,hash-access ,(length seq) (pack-sequence ,seq)) #\Tab ,seq))

(defun perform-work (stream)
  (declare (optimize (speed 3) (safety 0) (debug 0)))
  (with-smart-dna-hash (hash-access hash-loop :vector-threshold 16777216)
      (1 2 3 4 6 12 18)
    (macrolet ((incf-hash-element (seq-length key)
                 `(incf (hash-access ,seq-length ,key))))
      (with-dna-analyzed (stream incf-hash-element :block-size 655350) 1 2 3 4 6 12 18)
      (with-percentage (hash-loop (seqs-1 1) (seqs-2 2))
        (values (list seqs-1 seqs-2)
                (list (obtain-seq-count hash-access "GGT")
                      (obtain-seq-count hash-access "GGTA")
                      (obtain-seq-count hash-access "GGTATT")
                      (obtain-seq-count hash-access "GGTATTTTAATT")
                      (obtain-seq-count hash-access "GGTATTTTAATTTATAGT")))))))

(defun print-results (seq-freqs seq-counts)
  (labels ((compare (a b)
             (cond ((> (second a) (second b)) t)
                   ((< (second a) (second b)) nil)
                   (t (string< (first a) (first b)))))
           (print-freq (freq)
             (format t "~{~{~a ~3$~}~%~}~%" (sort freq #'compare))))
    (mapc #'print-freq seq-freqs)
    (format t "~{~{~a~c~a~}~%~}" seq-counts)))


(defun main ()
  (with-open-file (input-s #p"/dev/stdin" :external-format :iso-8859-1)
    (multiple-value-bind (freqs counts)
        (perform-work input-s)
      (print-results freqs counts))))


(in-package :cl-user)

(defun main ()
  (k-nucleotide::main))
