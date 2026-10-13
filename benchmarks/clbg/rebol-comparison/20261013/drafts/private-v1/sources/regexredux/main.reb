Rebol [Title: "CLBG regexredux baseline source proposal"]
io: do (join system/script/path %../../support/clbg_io.reb)
rx: do (join system/script/path %../../support/clbg_regex.reb)

count-matches: func [pattern [binary!] subject [binary!] /local position hit count] [
    position: 1
    count: 0
    while [not none? hit: rx/find pattern subject position] [
        if hit/2 <= hit/1 [io/fail "Unexpected empty match in selected counting pattern"]
        count: count + 1
        position: hit/2
    ]
    count
]

replace-all: func [pattern [binary!] replacement [binary!] subject [binary!]
    /local result position hit] [
    result: make binary! (length? subject)
    position: 1
    while [not none? hit: rx/find pattern subject position] [
        if hit/2 <= hit/1 [io/fail "Unexpected empty match in selected substitution pattern"]
        append result (copy/part (at subject position) (hit/1 - position))
        append result replacement
        position: hit/2
    ]
    append result (copy at subject position)
    result
]

variants: [
    "agggtaaa|tttaccct"
    "[cgt]gggtaaa|tttaccc[acg]"
    "a[act]ggtaaa|tttacc[agt]t"
    "ag[act]gtaaa|tttac[agt]ct"
    "agg[act]taaa|ttta[agt]cct"
    "aggg[acg]aaa|ttt[cgt]ccct"
    "agggt[cgt]aa|tt[acg]accct"
    "agggta[cgt]a|t[acg]taccct"
    "agggtaa[cgt]|[acg]ttaccct"
]
substitutions: [
    "tHa[Nt]" "<4>"
    "aND|caN|Ha[DS]|WaS" "<3>"
    "a[NSt]|BY" "<2>"
    "<[^>]*>" "|"
    "\|[^|][^|]*\|" "-"
]
input: io/read-input
sequence: replace-all (io/ascii ">.*\n|\n") #{} input
output: make binary! 1024
foreach pattern variants [
    append output io/ascii rejoin [pattern " " (count-matches (io/ascii pattern) sequence) "^/"]
]
replaced: sequence
foreach [pattern replacement] substitutions [
    replaced: replace-all (io/ascii pattern) (io/ascii replacement) replaced
]
append output io/ascii rejoin ["^/" (length? input) "^/" (length? sequence) "^/" (length? replaced) "^/"]
io/write-output output
