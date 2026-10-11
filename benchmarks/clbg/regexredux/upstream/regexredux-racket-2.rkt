#lang racket/base

;;; The Computer Language Benchmarks Game
;;; https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

;;; based on a version by by Anthony Borla
;;; regex-dna program contributed by Matthew Flatt
;;; converted from regex-dna program
;;; Parallelized by Gustavo Massaccesi, 2018


(require racket/port
         racket/place
         racket/list)

;; -------------------------------

(define VARIANTS
  '(#"agggtaaa|tttaccct" #"[cgt]gggtaaa|tttaccc[acg]" #"a[act]ggtaaa|tttacc[agt]t"
    #"ag[act]gtaaa|tttac[agt]ct" #"agg[act]taaa|ttta[agt]cct" #"aggg[acg]aaa|ttt[cgt]ccct"
    #"agggt[cgt]aa|tt[acg]accct" #"agggta[cgt]a|t[acg]taccct" #"agggtaa[cgt]|[acg]ttaccct"))


(define IUBS
  '((#"tHa[Nt]" #"<4>") (#"aND|caN|Ha[DS]|WaS" #"<3>") (#"a[NSt]|BY" #"<2>")
    (#"<[^>]*>" #"|") (#"\\|[^|][^|]*\\|" #"-")))

;; -------------------------------

(define (ci-byte-regexp s)
  (byte-regexp (bytes-append #"(?i:" s #")")))

;; -------------------------------

(define (match-count str rx offset cnt)
  (let ([m (regexp-match-positions rx str offset)])
    (if m
        (match-count str rx (cdar m) (add1 cnt))
        cnt)))

;; -------------------------------

(define (make-counting-place)
  (place ch
    (define filtered (place-channel-get ch))
    (define in (place-channel-get ch))
    (define out (map (lambda (i)
                       (match-count filtered (ci-byte-regexp i) 0 0))
                     in))
    (place-channel-put ch out)))

;; -------------------------------
;; -------------------------------

(module+ main
   ;; Load sequence
  (define orig (port->bytes))
  (define filtered (regexp-replace* #rx#"(?:>.*?\n)|\n" orig #""))

  ;; Create the places and launch the regexp counts
  ;; Since it is not possible to split the replacement part,
  ;; it's faster to use two places instead of three. 
  (define VARIANTS1 (drop-right VARIANTS 4))
  (define VARIANTS2 (take-right VARIANTS 4))

  (define place/ch1 (make-counting-place))
  (place-channel-put place/ch1 filtered)
  (place-channel-put place/ch1 VARIANTS1)

  (define place/ch2 (make-counting-place))
  (place-channel-put place/ch2 filtered)
  (place-channel-put place/ch2 VARIANTS2)
    
  ;; Perform regexp replacements while the places are running
  (define replaced
          (for/fold ([sequence filtered]) ([IUB IUBS])
            (regexp-replace* (byte-regexp (car IUB)) sequence (cadr IUB))))

  ;; Collect the results of the regexp counts
  (define count1 (place-channel-get place/ch1))
  (define count2 (place-channel-get place/ch2))


  ;; Print regexp counts
  (for ([i (in-list VARIANTS)]
        [j (in-list (append count1 count2))])
    (printf "~a ~a\n" i j))

  ;; Print statistics
  (printf "\n~a\n~a\n~a\n"
          (bytes-length orig) (bytes-length filtered) (bytes-length replaced))

  )
