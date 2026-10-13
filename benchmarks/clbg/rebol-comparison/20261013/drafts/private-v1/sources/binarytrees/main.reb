Rebol [Title: "CLBG binarytrees baseline source proposal"]
io: do (join system/script/path %../../support/clbg_io.reb)

make-tree: func [depth [integer!] /local node] [
    node: copy [none none]
    if depth > 0 [
        poke node 1 (make-tree (depth - 1))
        poke node 2 (make-tree (depth - 1))
    ]
    node
]

check-tree: func [node [block!]] [
    either none? first node [1] [1 + ((check-tree first node) + (check-tree second node))]
]

n: io/size
minimum-depth: 4
maximum-depth: max (minimum-depth + 2) n
stretch-depth: maximum-depth + 1
output: make binary! 512
stretch: make-tree stretch-depth
append output io/ascii rejoin ["stretch tree of depth " stretch-depth "^- check: " (check-tree stretch) "^/"]
stretch: none
long-lived: make-tree maximum-depth
depth: minimum-depth
while [depth <= maximum-depth] [
    iterations: to integer! (2 ** (maximum-depth - depth + minimum-depth))
    total: 0
    loop iterations [total: total + (check-tree make-tree depth)]
    append output io/ascii rejoin [iterations "^- trees of depth " depth "^- check: " total "^/"]
    depth: depth + 2
]
append output io/ascii rejoin ["long lived tree of depth " maximum-depth "^- check: " (check-tree long-lived) "^/"]
io/write-output output
