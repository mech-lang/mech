#!/usr/bin/awk -f

# Generate grammar.md from parser.rs
# Usage: `./extract_peg.awk < src/parser.rs > grammar.md`

BEGIN {
    TITLE = 0
    PEG = 1
    previous = TITLE
    print "## Mech Grammar\n"
    print strftime("Generated at [%m/%d/%Y %H:%M:%S]\n", systime())
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
    gsub(/^\/\/\s*/, "")
    print "\n"$0"\n"
    previous = TITLE
}

END {
    if (previous == PEG) {
        print "```"
    }
}
