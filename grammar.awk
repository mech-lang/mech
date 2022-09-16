#!/usr/bin/awk -f

# Generate grammar.md from parser.rs
# Usage: `./grammar.awk < src/parser.rs > grammar.md`

BEGIN {
    TITLE = 0
    PEG = 1
    previous = TITLE
    print strftime("# Mech Grammar (%m/%d/%Y %H:%M:%S)", systime())
    print 
    print "| Symbol |   Semantics                                         |"
    print "|:------:|:----------------------------------------------------|"
    print "|  \"abc\" | input matches string literal \"abc\" (terminal)       |"
    print "|  p*    | input matches `p` for 0 or more times (repetition)  |"
    print "|  p+    | input mathces `p` for 1 or more times (repetition)  |"
    print "|  p?    | input mathces `p` for 0 or 1 time (optional)        |"
    print "| p1, p2 | input matches `p1` followed by `p2` (sequence)      |"
    print "| p1\\|p2 | input matches `p1` or `p2` (ordered choice)         |"
    print "|  !!p   | input matches `p`; never consume input (peek)       |"
    print "|  !p    | input doesn't match `p`; never consume input (peek) |"
    print "| (...)  | common grouping                                     |"
    print "| <...>  | labeled grouping                                    |"
    print
}

# match against PEG
/^\/\/\s*[^ ]+\s*::=/ {
    if (previous == TITLE) {
        print "```"
    }
    gsub(/^\/\/\s*/, "")
    print
    previous = PEG
}

# match against markdown title
/^\/\/\s*#{3,6}[^#]/ {
    if (previous == PEG) {
        print "```"
    }
    gsub(/^\/\/\s*#/, "")
    print "\n"$0"\n"
    previous = TITLE
}

END {
    if (previous == PEG) {
        print "```"
    }
}
