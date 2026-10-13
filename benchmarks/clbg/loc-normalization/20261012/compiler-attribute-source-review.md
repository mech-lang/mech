# Written compiler annotation ownership

This additive proposal follows the parent-approved canonical rule: each written
meaningful C# attribute group, GNU `__attribute__((...))` group, or Nim pragma
block counts one separately from its attached declaration or function header.
Settings and arguments are structural. Plain keyword modifiers remain in the
header. Compiler-generated declarations do not acquire source units.

`compiler_attributes.py` recognizes balanced GNU double-parenthesized groups
with explicit known compiler settings. It retains offset/newline positions and
proposes one event for each actual group. The grammar-only mask lets an attached
type alias or function header retain its normal declaration ownership. An
annotation inside a preprocessor definition, unknown annotation name, or
malformed group retains a source-specific review gate; no generated annotation
is inferred from a macro call.

Actual old-source evidence includes `mandelbrot-gcc-1.c` lines 15–16, where two
separately written vector-size groups attach to two aliases, and `nbody-gcc-4.c`
line 17, where the named vector alias was previously unproven. These are general
annotation forms; no benchmark identity or input controls the parser. The
working fifteen focused tests are unexecuted until an immutable mutex-held
fixture pass. Existing jobs, sources, counts and timing records stay intact.

The same review found `binarytrees-gcc-2.c` line 13's explicit
`typedef intptr_t intnative_t` after its `stdint.h` include. The narrower old
contract included only fixed-width integers. Static native compiler headers
provide semantic evidence for pointer/max/least/fast integer names:
`/Library/Developer/CommandLineTools/usr/lib/clang/15.0.0/include/stdint.h`
lines 118–121, 216–219, 290–305; the native SDK `inttypes.h` line 229 includes
`stdint.h`. Explicit contracts now cover those standard integer spellings only
when the exact include token is proven. This static receipt neither hashes the
headers nor claims original-build provenance. Conditional imports, missing or
invented headers and arbitrary type spellings stay gated.

The C/C++ job09 review separately identified standard `size_t` groups and
`PCRE2_SIZE` groups. Minimal explicit contracts now cover `stdio.h`, `stdlib.h`,
`string.h` and `pcre2.h` for `size_t`, and the exact `pcre2.h` spelling for
`PCRE2_SIZE`. Static SDK evidence is `stdio.h`'s `_stdio.h` delegation,
`_stdio.h` lines 71–77, the standard size typedef header lines 28–32,
`stdlib.h` lines 70–74 and `string.h` lines 65–67. The viewed PCRE2 header
includes `stdlib.h` and `inttypes.h` at lines 88–90 and defines `PCRE2_SIZE`
as `size_t` at line 536. These are semantic contracts and later locked source
pins still must reconcile. No arbitrary `PCRE2_` prefix, missing include,
POSIX transitively exposed `off_t`, or unknown pointer type is promoted.

Even a recognized setting cannot make independent executable annotation
arguments opaque: brace/semicolon callback bodies and explicit binding
operators retain the annotation-owner gate. Plain literal or symbolic
settings remain structural within their one written meaningful group.
