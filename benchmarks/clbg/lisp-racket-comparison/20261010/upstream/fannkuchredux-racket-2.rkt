#lang racket/base

;;; The Computer Language Benchmarks Game
;;; https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

;; Written by Dima Dorfman, 2004
;; Slightly improved by Sven Hartrumpf, 2005-2006
;; Ever-so-slightly tweaked for MzScheme by Brent Fulgham
;; PLT-ized for v4.0 by Matthew
;; Updated by Danny Yoo and Matthias Felleisen
;; Optimized by Gustavo Massaccesi, 2013

(require (for-syntax (only-in racket/base
                              lambda
                              syntax
                              syntax-case
                              make-rename-transformer
                              #%app)))
(require racket/unsafe/ops)
(require racket/cmdline)

(define-sequence-syntax unsafe-in-fxrange 
  (lambda () #'in-fxrange/proc) 
  (lambda (stx) 
    (syntax-case stx () 
      [[(d) (_ nat)] 
       #'[(d) 
          (:do-in ([(n) nat])
                  #f
                  ([i 0])
                  (unsafe-fx< i n)
                  ([(d) i])
                  #t
                  #t
                  [(unsafe-fx+ 1 i)])]]))) 

(define (unsafe-in-fxrange/proc n) 
  (make-do-sequence (lambda () (values (lambda (x) x)
                                       (lambda (x) (unsafe-fx+ 1 x))
                                       0
                                       (lambda (x) (unsafe-fx< x n))
                                       #f
                                       #f)))) 


(define-syntax-rule (define/0st-bool (name arg0 rest ...) body ...)
  (begin
    (define-syntax-rule (name arg0/v rest ...)
      (if arg0/v (name/t rest ...) (name/f rest ...)))
    (define (name/t rest ...) (let ([arg0 #t]) body ...))
    (define (name/f rest ...) (let ([arg0 #f]) body ...))
    ))

(define (fannkuch n)
  (let ([pi (for/vector #:length n ([i (unsafe-in-fxrange n)]) i)]
        [tmp (make-vector n)]
        [count (make-vector n)])
    (define/0st-bool (loop even-parity? flips r checksum n pi tmp count)
      (for ([i (unsafe-in-fxrange r)])
        (unsafe-vector-set! count i (unsafe-fx+ 1 i)))
      (let* ([next-flips (count-flips pi tmp)]
             [flips2 (unsafe-fxmax next-flips flips)]
             [next-checksum (if even-parity?
                                (unsafe-fx+ checksum  next-flips)
                                (unsafe-fx- checksum next-flips))])
        (let loop2 ([r 1])
          (if (unsafe-fx= r n)
              (values flips2 next-checksum)
              (let ([perm0 (unsafe-vector-ref pi 0)])
                (for ([i (unsafe-in-fxrange r)])
                  (unsafe-vector-set! pi i (unsafe-vector-ref pi (unsafe-fx+ 1 i))))
                (unsafe-vector-set! pi r perm0)
                (unsafe-vector-set! count r (unsafe-fx- (unsafe-vector-ref count r) 1))
                (if (unsafe-fx= (unsafe-vector-ref count r) 0)
                    (loop2 (unsafe-fx+ 1 r))
                    (loop (not even-parity?) 
                          flips2 
                          r 
                          next-checksum 
                          n
                          pi 
                          tmp 
                          count)))))))
    (loop #t 0 n 0  n pi tmp count)))

(define (count-flips pi rho)
  (vector-copy! rho 0 pi)
  (let loop ([k 0])
    (if (unsafe-fx= (unsafe-vector-ref rho 0) 0)
        k
        (let loop2 ([i 0]
                    [j (unsafe-vector-ref rho 0)])
          (if (unsafe-fx> j i)
              (begin 
                (vector-swap! rho i j)
                (loop2 (unsafe-fx+ 1 i) (unsafe-fx- j 1)))
              (loop (unsafe-fx+ 1 k)))))))

(define-syntax-rule (vector-swap! v i j)
  (let ([t (unsafe-vector-ref v i)])
    (unsafe-vector-set! v i (unsafe-vector-ref v j))
    (unsafe-vector-set! v j t)))

(command-line #:args (n)
              (define-values (answer checksum)
                (fannkuch (string->number n)))
              (printf "~a\nPfannkuchen(~a) = ~a\n" 
                      checksum
                      n 
                      answer))
