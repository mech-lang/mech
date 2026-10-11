;; The Computer Language Benchmarks Game
;;   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
;;
;;   contributed by Currell Berry
;;
;; Based on Java submission #1

(defpackage :knucleotide2
  (:use :cl))

(in-package :knucleotide2)

(declaim (optimize (speed 3) (safety 0) (space 0) (debug 0)))
(declaim (inline get-key))

;; simple thread runner implementation
;; we have a semaphore
(defparameter *my-tr-available-thread-semaphore* nil)
;; each time a thread finishes up, we increment this semaphore
;; the main thread waits on the semaphore, whenever it goes above 0 it finds another task if one is available and
;; starts it up


(defparameter *my-tr-task-remaining-count* 0)
(declaim (type fixnum *my-tr-task-remaining-count*))
(defparameter *my-tr-completed-task-mutex* nil)
(defparameter *my-tr-task-completed-cv* nil)

;;writing to status and result should only be done in my-tr-run
(defstruct my-task
  (mylambda) ; the thing to run
  (status nil) ; nil or t
  (result nil))

(defparameter *my-task-list* #())
(declaim (type vector *my-task-list*))

;; must have set up populated *my-task-list* first
;; each time a thread becomes available, then we run the next task
(defun my-tr-run (threadcount)
  (declare (type fixnum threadcount))
  (setf *my-tr-available-thread-semaphore* (sb-thread:make-semaphore :count threadcount))
  (setf *my-tr-completed-task-mutex* (sb-thread:make-mutex)) 
  (setf *my-tr-task-remaining-count* (length *my-task-list*)) 
  (setf *my-tr-task-completed-cv* (sb-thread:make-waitqueue)) 
  (loop for taskindex from 0 below (length *my-task-list*) do
       (sb-thread:wait-on-semaphore *my-tr-available-thread-semaphore*)
       (let ((thetask (elt *my-task-list* taskindex)))
         (sb-thread:make-thread (lambda () (let ((results
                                                  (funcall (my-task-mylambda thetask))))
                                             (setf (my-task-result thetask) results)
                                             (setf (my-task-status thetask) t)
                                             (sb-thread:signal-semaphore *my-tr-available-thread-semaphore*)
                                             (sb-thread:with-mutex (*my-tr-completed-task-mutex*)
                                               (decf *my-tr-task-remaining-count*)
                                               (sb-thread:condition-notify *my-tr-task-completed-cv*)
                                               )
                                             )))))
  (loop
     (sb-thread:with-mutex (*my-tr-completed-task-mutex*)
       (if (eql *my-tr-task-remaining-count* 0)
           (return)
           (sb-thread:condition-wait *my-tr-task-completed-cv* *my-tr-completed-task-mutex*)))))

(defconstant CODES  #(-1 0 -1 1 3 -1 -1 2)) 
(defconstant NUCLEOTIDES #(#\A #\C #\G #\T)) 

(defun hash-function (x)
  (declare (type (unsigned-byte 64) x))
  x)

(defstruct result
  (outmap (make-hash-table
           :test 'eql
           :hash-function #'hash-function
           :rehash-size 2.0
           :rehash-threshold 0.7))
  (keylength 0))

(defun create-fragment-tasks (sequence mfragment-lengths)
  (declare (type (simple-array (unsigned-byte 8) (*)) sequence))
  (let ((tasks (make-array 46 :fill-pointer 0)))
    (loop for fragmentLength across mfragment-lengths do
         (loop for i of-type (unsigned-byte 32) from 0 below fragmentLength do
              (let ((offset i)
                    (mfragmentlength fragmentLength))
                (vector-push-extend
                 (make-my-task :mylambda
                               (lambda ()
                                 (declare (type (unsigned-byte 32) mfragmentlength)
                                          (type (unsigned-byte 32) offset))
                                 (create-fragment-map sequence offset mfragmentlength)
                                 ))
                 tasks))))
    tasks))

(defun create-fragment-map (sequence offset fragmentLength)
  (declare (type (simple-array (unsigned-byte 8) (*)) sequence)
           (type (unsigned-byte 32) offset)
           (type (unsigned-byte 32) fragmentLength))
  (let* ((res (make-result :keylength fragmentLength))
         (mymap (result-outmap res))
         (lastIndex (+ (- (length sequence) fragmentLength) 1)))
    (declare (type (unsigned-byte 32) lastIndex))
    (loop
       for index of-type (unsigned-byte 32) from offset below lastIndex by fragmentLength
       do
         (let* ((key (get-key sequence index fragmentLength))
                (value (gethash key mymap 0)))
           (declare (type (unsigned-byte 64) key)
                    (type (unsigned-byte 32) value))
           (setf (gethash key mymap 0) (the (unsigned-byte 32) (+ value 1)))))
    res))

(defun sum-two-maps (result1 result2)
  (loop for key2 of-type (unsigned-byte 64) being the hash-keys  of (result-outmap result2)
     using (hash-value value2) do
       (setf (gethash key2 (result-outmap result1)) (+ (the (unsigned-byte 32) (gethash key2 (result-outmap result1) 0)) (the (unsigned-byte 32) value2))))
  result1
  )

(defun write-frequencies (totalCount frequencies)
  (let ((freq (make-array (hash-table-count (result-outmap frequencies)) :fill-pointer 0 :element-type 'cons)))
    (loop for key being the hash-keys of (result-outmap frequencies)
       using (hash-value cnt) do
         (let ((nentry (cons (key-to-string key (result-keylength frequencies)) cnt)))
           (vector-push-extend nentry freq)))
    (sort freq (lambda (x y) (> (cdr x) (cdr y))))
    (let ((outstr
           (apply #'concatenate 
                  (append (list 'string)
                          (loop for index from 0 below (length freq)
                                           for (key . value) = (elt freq index)
                                           collect 
                                             (format nil "~a ~,3f~%" key (/ (* value 100.0) totalCount)))))))
      outstr)))

(defun write-count (tasks nucleotideFragment)
  (let* ((key (to-codes-new (map '(vector (unsigned-byte 8)) #'char-code nucleotideFragment) (length nucleotideFragment)) )
         (k (get-key key 0 (length nucleotideFragment)))
         (count 0))
    (loop for task across tasks 
       for result = (my-task-result task) do
         (if (eql (result-keylength result) (length nucleotideFragment))
             (setf count (+ count (gethash k (result-outmap result) 0)))))
    (format nil "~a~c~a~%" count #\tab nucleotideFragment)))

(defun key-to-string (key length)
  (declare (type (unsigned-byte 64) key)
           (type (unsigned-byte 32) length))
  (let ((res (make-string length)))
    (loop for i from 0 below length do
         (setf (elt res (- length i 1)) (elt NUCLEOTIDES (logand key #x3)))
         (setf key (ash key -2)))
    res))

(defun get-key (arr offset length)
  (declare (type (simple-array (unsigned-byte 8)) arr)
           (type (unsigned-byte 32) offset)
           (type (unsigned-byte 32) length))
  (let ((key 0))
    (declare (type (unsigned-byte 64) key))
    (loop for i of-type (unsigned-byte 32) from offset below (+ offset length) do
         (setf key (the fixnum (+ (the fixnum (* key 4)) (the fixnum (elt arr i))))))
    key))

(defun to-codes-new (sequence length)
  (declare (type (simple-array (unsigned-byte 8) (*)) sequence)
           (type (unsigned-byte 32) length)
           )
  (let ((result (make-array length :element-type '(unsigned-byte 8))))
    (loop for i of-type (unsigned-byte 32) from 0 below length do
         (setf (elt result i) (elt CODES (logand (elt sequence i) #x7))))
    result))

(defconstant FRAGMENT-LENGTHS  #(1 2 3 4 6 12 18)) 
(defconstant NUCLEOTIDE-FRAGMENTS #("GGT" "GGTA" "GGTATT" "GGTATTTTAATT"
                                           "GGTATTTTAATTTATAGT" ))
(defconstant DNA_THREE_IN_BYTES (map '(simple-array (unsigned-byte 8) (*)) #'char-code ">THREE"))
(defconstant NEWLINE-CODE  (map '(simple-array (unsigned-byte 8) (*)) #'char-code "
")) 

(defun read-ascii-file-to-binary-array (filename)
  (with-open-stream (stream (open filename :element-type '(unsigned-byte 8)))
    (let* ((buffer
            (make-array (file-length stream)
                        :element-type
                        '(unsigned-byte 8))))
      (read-sequence buffer stream)
      buffer)))

(defun read-in-data-chunked (ifilename)
  (let ((bytes (read-ascii-file-to-binary-array ifilename)))
    (declare (type (simple-array (unsigned-byte 8) (*)) bytes))
    ;;we have read all the data!
    ;;we have to doctor the array so that it no longer has newlines and junk
    ;;  (print-range bytes 0 100)
    (let* ((threestart (search DNA_THREE_IN_BYTES bytes))
           (realstart (the (unsigned-byte 32) (search NEWLINE-CODE bytes :start2 threestart)))
           (writeposition 0))
      (declare (type (unsigned-byte 32) threestart)
               (type (unsigned-byte 32) realstart)
               (type (unsigned-byte 32) writeposition)) 
      (loop for pos from realstart below (length bytes) do
           (let ((nchar (aref bytes pos)))
             (if (not (eql nchar 10)) ; newline
                 (progn
                   (setf (aref bytes writeposition) (aref bytes pos))
                   (incf writeposition))
                 )))
      (let ((outarr (make-array writeposition :element-type '(unsigned-byte 8))))
        (declare (type (simple-array (unsigned-byte 8) (*)) outarr))
        (loop for i from 0 below writeposition do
             (setf (aref outarr i) (aref bytes i)))
        (return-from read-in-data-chunked outarr)))))

(defun main ()
  (let* ((pifile #p"/dev/stdin")
         (msequenceraw (read-in-data-chunked pifile))
         (msequence (to-codes-new msequenceraw (length msequenceraw))))
;    (setf *tbytes* msequenceraw)
    (setf *my-task-list* (create-fragment-tasks msequence FRAGMENT-LENGTHS))
    (my-tr-run 4)
    (format t "~a~%" (write-frequencies (length msequence) (my-task-result (elt *my-task-list* 0))))
    
    (format t "~a~%" (write-frequencies (- (length msequence) 1) (sum-two-maps
                                                       (my-task-result (elt *my-task-list* 1))
                                                       (my-task-result (elt *my-task-list* 2))
                                                                  )))
    (loop for nucleotide-fragment across NUCLEOTIDE-FRAGMENTS do
         (princ (write-count *my-task-list*  nucleotide-fragment)))
))

(in-package :cl-user)

(defun main ()
  (knucleotide2::main))
