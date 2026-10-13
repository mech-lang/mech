Rebol [Title: "Counted general binary PCRE2 finder facade proposal"]
generic-regex-commands: do (join system/script/path %clbg_primitives.reb)
context [
    commands: generic-regex-commands
    find: func [pattern [binary!] subject [binary!] start [integer!]] [commands/pcre-find pattern subject start]
]
