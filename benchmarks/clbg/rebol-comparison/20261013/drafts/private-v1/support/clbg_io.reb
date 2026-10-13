Rebol [Title: "Counted CLBG byte IO and decimal formatting proposal"]
generic-io-commands: do (join system/script/path %clbg_primitives.reb)

context [
    commands: generic-io-commands
    fail: func [message [string!]] [do make error! message]

    size: func [/local arguments value] [
        arguments: system/options/args
        if any [none? arguments empty? arguments] [fail "Missing numeric workload size"]
        value: to integer! first arguments
        if value < 0 [fail "Negative workload size"]
        value
    ]

    read-input: func [/local result chunk] [
        result: make binary! 32768
        while [chunk: commands/read-stdin-chunk not none? chunk] [
            if ((length? result) + (length? chunk)) > 2147483583 [fail "Stdin exceeds signed byte series limit"]
            append result chunk
        ]
        result
    ]
    write-output: func [data [binary!]] [commands/write-stdout data]
    ascii: func [value [string!]] [to binary! value]

    fixed: func [value [decimal!] digits [integer!]] [to string! commands/format-fixed value digits]

    lines: func [data [binary!] /local rows start position byte] [
        rows: make block! 16
        start: 1
        position: 1
        while [position <= length? data] [
            byte: pick data position
            either any [byte = 10 byte = 13] [
                append/only rows (copy/part (at data start) (position - start))
                if all [byte = 13 position < length? data (pick data (position + 1)) = 10] [
                    position: position + 1
                ]
                start: position + 1
            ] []
            position: position + 1
        ]
        if start <= length? data [append/only rows (copy at data start)]
        rows
    ]
]
