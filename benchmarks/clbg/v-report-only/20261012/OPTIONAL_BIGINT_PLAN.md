# Optional published pure-V big integer compatibility

The source imported by the published pi program is
[hanabi1224/vlang-big-integer](https://github.com/hanabi1224/vlang-big-integer/tree/dae1213e3b7576b40bf84a5c45a25748a74ef3c0),
commit `dae1213e3b7576b40bf84a5c45a25748a74ef3c0`. Original files, example,
primitive tests, MIT license, API commit metadata and tree are retained under
`upstream/vlang-big-integer`. It implements ordinary signed integer arithmetic
with32-bit limbs in V. It is not a GMP binding and contains no called pi kernel;
the benchmark spigot remains entirely in its application source.

No module has been installed, modified or built. Existing sealed libraries and
every Lua/LuaJIT GMP module remain untouched. If used, copy only the original
library's module files into a new private, revisioned module directory, preserving
the immutable upstream tree. The parent controls V's isolated module path and
pins original/adapted files, compiler and binary before qualification.

The main anticipated migration is the older fallible `div_mod` declaration and
recursive propagation. V0.5.2 distinguishes optional absence from an error:
adapt the checked division API to a Result and preserve real division-by-zero
errors with an explicit return. The existing operator wrapper's documented
behavior must remain separately described; do not silently change quotient,
remainder sign, operand aliasing or arithmetic order to improve a benchmark.
Other compatibility repairs must be limited to equivalent general syntax/API
changes, each recorded against the exact original file.

The package must pass independent primitive tests: constructors/decimal strings,
zero and signs, conversion boundaries, add/subtract with carry/borrow, multiplication,
comparisons, long and small divisors, exact/inexact quotients, negative quotient
and remainder conventions, division by zero and immutable operand preservation.
Original package tests alone are insufficient. The pi benchmark must then pass
the same small/tail cases, two-extract trace and entire10000-digit output as the
standard math.big form. Any measured package result needs a distinct honest
library profile and fresh matched Mech controls; no standard-library sample is
relabeled or reused.

Primary syntax/API references are the pinned V0.5.2
[documentation](https://github.com/vlang/v/blob/7647ce1c6fad63b5578bc07883139906de74b2f8/doc/docs.md)
and math.big source saved under `upstream/v-api`. The optional module's standard
arithmetic library code can be excluded from application LOC only after this
general-purpose scope/compatibility review. Its example pi source is never part
of the imported library or an excluded application helper.

This plan makes no speed or LOC claim. A future public GMP primitive/lifetime
binding would be a separately reviewed library profile, not a silent replacement
of this pure-V package.
