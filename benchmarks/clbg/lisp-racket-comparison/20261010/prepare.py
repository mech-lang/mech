"""Preserve upstream sources and label portable, common-workload corrections."""
import hashlib
import json
import re
from pathlib import Path

D = Path(__file__).resolve().parent


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def replace(text, old, new):
    assert old in text, old
    return text.replace(old, new)


def main():
    manifest = json.loads((D / 'manifest.json').read_text())
    corrections = []
    for row in manifest['programs']:
        text = Path(row['source']).read_text()
        original = text
        name, bench, variant = row['id'], row['benchmark'], row['variant']
        notes = []
        if name == 'binarytrees-sbcl-1':
            start, end = text.index('(defun build-btree'), text.index('(defun loop-depths')
            text = text[:start] + '''(defun build-btree (item depth)
  (declare (fixnum item depth) (ignore item))
  (if (zerop depth)
      (cons nil nil)
      (cons (build-btree 0 (1- depth))
            (build-btree 0 (1- depth)))))

(defun check-node (node)
  (declare (values fixnum))
  (if (car node)
      (+ 1 (check-node (car node)) (check-node (cdr node)))
      1))

''' + text[end:]
            notes.append('Use one separately allocated two-slot cons for every leaf and interior node, rather than an item-bearing one-slot leaf and double-cons interior. Retain construction before traversal, tree counts and the long-lived root. Constructor and walker remain ordinary Lisp source and count toward LOC.')
        if bench == 'fasta' and row['language'] == 'lisp' and variant in (3, 6):
            start, end = text.index('(defun cumsum'), text.index('(defparameter *amino-acids-syms*')
            text = text[:start] + '''(defun cumsum (probs)
  (let ((sum 0.0d0))
    (declare (double-float sum))
    (map 'list (lambda (p)
                 (declare (double-float p))
                 (incf sum p))
         probs)))

(defun make-cprob (probs)
  (make-array (length probs)
              :element-type 'double-float
              :initial-contents (cumsum probs)))

''' + text[end:]
            # Convert literal probabilities to binary64 without changing their decimal values.
            for symbol in ('*amino-acids-cprobs*', '*homo-sapiens-cprobs*'):
                start = text.index('(defparameter ' + symbol)
                end = text.index('\n\n', start)
                part = text[start:end]
                part = re.sub(r'(?<![\w.])(0\.\d+)(?![\w.])', r'\1d0', part)
                text = text[:start] + part + text[end:]
            text = text.replace('(simple-array fixnum (*)) amino-acids-cprobs', '(simple-array double-float (*)) amino-acids-cprobs')
            text = replace(text, '(type fixnum p)', '(type double-float p)')
            text = replace(text, '(declare (values fixnum))\n', '(declare (values double-float))\n')
            text = replace(text, '(setf r (mod (+ (the (integer 0 542655936) (* r ia)) ic) +im+))))',
                '(setf r (mod (+ (the (integer 0 542655936) (* r ia)) ic) +im+))\n    (/ (float r 1.0d0) (float +im+ 1.0d0))))')
            notes.append('Replace single-float, ceil-scaled integer probability thresholds with unscaled cumulative binary64 probabilities. Every fresh LCG result is converted to binary64 and divided directly by IM. Retain published binary/linear lookup, buffering and all output.')
        if bench == 'knucleotide' and row['language'] == 'lisp':
            if variant in (2, 3):
                if variant == 2:
                    text = replace(text, 'for i from 0 to (the fixnum (1+ (- (length gene) k)))', 'for i from 0 below (length gene)')
                else:
                    text = replace(text, '(dotimes (i (the fixnum (- (length gene) k)))', '(dotimes (i (length gene))')
                    text = replace(text, 'for i from 0 to (the fixnum (- (length gene) k))', 'for i from 0 below (length gene)')
                text = replace(text, '(t (< (car v1) (car v2)))', '(t (if (= k 1) (char< (car v1) (car v2)) (< (car v1) (car v2))))')
                notes.append('Process every input byte for the rolling key so every width includes its final overlapping windows. Use character comparison for width-one frequency ties; retained growable hash table is cleared between seven complete maps.')
            if variant in (4, 5):
                text = replace(text, ':vector-threshold 16777216', ':vector-threshold 0')
                text = replace(text, ':rehash-size ,(expt 2 (1- length))', ':rehash-size 2.0')
                text = text.replace(':and :collect', ':collect')
                notes.append('Disable indexed histogram selection: all seven widths start as small ordinary growable hash maps with explicit gethash/incf updates. Use the general doubling growth factor rather than a width-derived huge allocation step. Preserve rolling packing, stream fusion and complete overlapping windows.')
            if variant in (1, 6):
                text = replace(text, '(sort freq (lambda (x y) (> (cdr x) (cdr y))))', '(sort freq (lambda (x y) (or (> (cdr x) (cdr y))\n                                (and (= (cdr x) (cdr y)) (string< (car x) (car y))))))')
                notes.append('Sort equal-frequency keys lexically. All offset tasks still populate their complete ordinary growable hash maps; each width partitions all valid overlapping positions and count queries aggregate every offset partition.')
        if bench == 'knucleotide' and row['language'] == 'racket':
            comparison = '(lambda (a b) (or (> (cdr a) (cdr b))\n                            (and (= (cdr a) (cdr b)) (string<? KEY-A KEY-B))))'
            comparison = comparison.replace('KEY-A', '(symbol->string (car a))' if variant == 1 else '(unhash (car a) len)')
            comparison = comparison.replace('KEY-B', '(symbol->string (car b))' if variant == 1 else '(unhash (car b) len)')
            text = replace(text, '(sort content > #:key cdr)', '(sort content ' + comparison + ')')
            notes.append('Sort equal-frequency keys lexically; retain all seven complete overlapping ordinary growable hash maps, symbol or packed integer keys, and every window lookup/update.')
        if bench == 'pidigits' and row['language'] == 'lisp' and variant in (3, 4):
            text = replace(text, '    (when (> num acc)\n      (go continue))\n', '')
            text = replace(text, '(truncate (+ (* num nth) acc) den)', '(floor (+ (* num nth) acc) den)')
            notes.append('Perform extract(3) and extract(4) on every attempted LFT step instead of skipping both when num > acc. Floor division handles the signed intermediate numerator. General SB-GMP integer operations remain the only library acceleration; all spigot control and output stay in Lisp.')
        if name == 'pidigits-racket-2':
            text = replace(text, '(ffi-lib "libgmp")', '(ffi-lib "/opt/homebrew/opt/gmp/lib/libgmp.10.dylib")')
            text = replace(text, '    (if (gmp _x0 > _x1)\n      (compose1!+loop)\n      (let ([d (extract-digit 3)])', '    (let ([d (extract-digit 3)])')
            text = replace(text, 'n))))))))))', 'n)))))))))')
            text = replace(text, '(defgmp tdiv_q', '(defgmp fdiv_q')
            text = replace(text, "#'(tdiv_q x y z)", "#'(fdiv_q x y z)")
            notes.append('Locate the installed macOS GMP library, use signed floor division and perform both LFT extracts on every attempt. All FFI declarations/macros and the spigot are included in Racket LOC; GMP supplies general arbitrary-precision arithmetic, not a digit executor.')
        if name == 'spectralnorm-racket-3':
            text = replace(text, '(fx* n stride) (fxmin N (fx* (fx+ n 1) stride))', '(fxquotient (fx* n N) k) (fxquotient (fx* (fx+ n 1) N) k)')
            notes.append('Use balanced integer partition boundaries to cover every row even when N is not divisible by the worker count. Retain forty complete matrix-vector products with coefficient evaluation inside the inner loop.')
        if bench == 'mandelbrot' and row['language'] == 'lisp' and variant in (1, 4):
            text = replace(text, '#+x86(defmacro escapes?', '#-x86-64(defmacro escapes?')
            notes.append('Enable the published ordinary scalar fallback on non-x86-64 hosts. No SSE instructions are translated or simulated; all recurrence, two-pixel packing and row scheduling remain in Lisp.')
        if name == 'regexredux-sbcl-4':
            text = replace(text, '(defconstant  +cpu-count+ (sysconf 84))', '(defconstant  +cpu-count+ 8)')
            start, end = text.index('(defun parts'), text.index('(defun replace-all')
            text = text[:start] + '''(defun parts (parts-num len)
  (loop for i below parts-num
        collect (cons (floor (* i len) parts-num)
                      (floor (* (1+ i) len) parts-num))))

''' + text[end:]
            text = replace(text, '(cl-ppcre:all-matches regex sequence)', '(cl-ppcre:all-matches (cl-ppcre:create-scanner regex :case-insensitive-mode t) sequence)')
            text = replace(text, '(cl-ppcre:all-matches reg sequence)', '(cl-ppcre:all-matches (cl-ppcre:create-scanner reg :case-insensitive-mode t) sequence)')
            notes.append('Replace the Linux sysconf constant with eight copy workers and balanced partitions that also handle fewer than eight matches. Count all nine regexes case-insensitively over the entire cleaned sequence; keep the five ordered case-sensitive substitutions. General CL-PPCRE supplies regex matching; the published Lisp replacement code is counted.')
        if name == 'revcomp-sbcl-2':
            text = replace(text, 'for line simple-string = (read-line in nil)', 'for line = (read-line in nil)')
            text = replace(text, ':if-exists :supersede', ':if-exists :append')
            notes.append('Allow the EOF sentinel NIL rather than declaring every read-line result a simple-string under safety-zero compilation, which removes the EOF check. Open the already-existing stdout device in append mode on macOS. Reverse/complement, full output and ordinary buffering are unchanged.')
        if text != original:
            identifier = name + '-faithful'
            target = D / 'sources' / (identifier + Path(row['source']).suffix)
            target.write_text(text)
            corrected = dict(row, id=identifier, key=bench + ':' + identifier, source=str(target), source_sha256=sha(target),
                             adaptations=notes, upstream_id=name, local_adaptation=True)
            corrections.append(corrected)
    (D / 'corrections.json').write_text(json.dumps(dict(status='corrections-pinned', manifest_sha256=sha(D / 'manifest.json'), programs=corrections), indent=2) + '\n')
    print('Pinned', len(corrections), 'separate corrections')


if __name__ == '__main__':
    main()
