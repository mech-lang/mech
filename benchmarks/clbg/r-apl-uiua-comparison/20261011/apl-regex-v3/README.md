# General PCRE2 count candidate for GNU APL

The frozen v2 sources collect matches through GNU APL's `⎕RE`. This additive
candidate counts with a separate general PCRE2 binding. Both APL sources retain
all nine patterns, the loop/table output, input cleanup, scalar lengths and all
five ordered complete substitutions. Their normalized application LOC is 33.
The v2 substitution module and every frozen v2 source/binary remain unchanged.

The new standard-library interface accepts arbitrary pattern/subject strings:

```apl
Matches←Pattern Count Subject
Matches←Pattern Count[1] Subject
Matches←Pattern Count[8 1] Subject
```

The first call uses default PCRE2 matching. Axis 1 requests its ordinary JIT.
A two-element axis supplies ordinary compile-option bits and JIT 0/1; in the
third example, 8 is `PCRE2_CASELESS`. It also supports flags such as multiline,
UTF and Unicode character properties. Invalid options/types/patterns report
ordinary APL errors.

The binding uses standard `pcre2_compile`, `pcre2_match` and
[`pcre2_next_match`](https://www.pcre.org/current/doc/html/pcre2_next_match.html).
That last API explicitly supports global counting and supplies correct
empty-match progress and character/newline boundaries. The wrapper retains one
reusable match-data frame and returns one integer. It creates no all-match
arrays or captured strings. Its source contains no benchmark patterns, record
parser, substitutions, table/length logic, benchmark dispatch or I/O.

The binding is built against the pinned installed PCRE2 10.49 32-bit library
and GNU APL 2.0's published native interface. Build/validation acquire the same
exclusive mutex as all benchmark runs:

```sh
python3 build_and_validate.py --wait-for-lock
python3 prepare.py
```

Validation includes canonical/adversarial complete benchmark outputs and general
count/substitution checks: empty and Unicode strings, nonoverlap, alternation,
escapes, captures/backreferences, lookahead/non-greedy empty matches, flags,
embedded NUL and malformed arguments. Full measurements are left to the parent
guarded harness; these sources have no admitted timing until that completes.
