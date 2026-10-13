Rebol [Title: "Counted explicit private general extension loader proposal"]
extension-path: get-env "MECH_CLBG_REBOL_PRIMITIVES"
if any [none? extension-path empty? extension-path (first extension-path) <> #"/"] [
    do make error! "Root-pinned absolute private extension path is required"
]
extension: import (to file! extension-path)
unless module? extension [do make error! "General extension did not return a module"]
foreach name [gmp-add gmp-subtract gmp-multiply gmp-floor-divide gmp-compare
    pcre-find read-stdin-chunk write-stdout format-fixed] [
    unless command? (get in extension name) [do make error! "Missing general extension command"]
]
extension
