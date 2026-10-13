Rebol [Title: "Counted general signed decimal arithmetic facade proposal"]
generic-big-commands: do (join system/script/path %clbg_primitives.reb)
context [
    commands: generic-big-commands
    from-int: func [value [integer!]] [to binary! form value]
    to-int: func [value [binary!]] [to integer! to string! value]
    add: func [a [binary!] b [binary!]] [commands/gmp-add a b]
    subtract: func [a [binary!] b [binary!]] [commands/gmp-subtract a b]
    multiply: func [a [binary!] b [binary!]] [commands/gmp-multiply a b]
    floor-divide: func [a [binary!] b [binary!]] [commands/gmp-floor-divide a b]
    compare: func [a [binary!] b [binary!]] [commands/gmp-compare a b]
]
