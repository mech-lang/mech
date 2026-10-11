#lang racket/base

;;; The Computer Language Benchmarks Game
;;; https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

;;; Derived from the Chicken variant by Sven Hartrumpf
;;; contributed by Matthew Flatt
;;; improved by Phil Nguyen:
;;; - use `cons` instead of struct `node`
;;; - removed the confirmed unneccessary field `val`
;;; - clean up with `define` instead of nested `let`
;;; - clean up with `for/sum` instead of generic `for/fold`
;;; *reset*

(require racket/cmdline)

#;(struct node (left right))
(define node cons)
(define node-left car)
(define node-right cdr)

(define (make d)
  (if (= d 0)
      (node #f #f)
      (let ([d2 (sub1 d)])
        (node (make d2) (make d2)))))

(define (check t)
  (if (node-left t)
      (+ 1 (check (node-left t)) (check (node-right t)))
      1))

(define (main n)
  (define min-depth 4)
  (define max-depth (max (+ min-depth 2) n))
  (define stretch-depth (+ max-depth 1))
  (printf "stretch tree of depth ~a\t check: ~a\n" stretch-depth (check (make stretch-depth)))
  (define long-lived-tree (make max-depth))
  (for ([d (in-range 4 (add1 max-depth) 2)])
    (define iterations (arithmetic-shift 1 (+ (- max-depth d) min-depth)))
    (printf "~a\t trees of depth ~a\t check: ~a\n"
            iterations
            d
            (for/sum ([_ (in-range iterations)])
              (check (make d)))))
  (printf "long lived tree of depth ~a\t check: ~a\n" max-depth (check long-lived-tree)))

(command-line #:args (n) 
              (main (string->number n)))
