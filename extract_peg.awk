#!/usr/bin/awk -f

# Generate grammar.md from parser.rs
# Usage: `./extract_peg.awk < src/parser.rs > grammar.md`

BEGIN {
    TITLE = 0
    PEG = 1
    previous = TITLE
    print strftime("# Mech Grammar (%m/%d/%Y %H:%M:%S)\n", systime())
    print "\
For now, the formal specification of the Mech grammar will be the Rust \
implementation. I will try to reflect that grammar in this document in \
[EBNF](https://en.wikipedia.org/wiki/Extended_Backus–Naur_form). Then this \
document can be used to generate Mech parsers in any number of languages.\
"
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
