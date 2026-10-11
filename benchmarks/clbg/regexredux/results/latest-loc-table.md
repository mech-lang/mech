# regexredux: latest measured endpoints

Apple M3; AC power; Low Power Mode off. Complete full workloads and stdout. Medians of three broad-sweep samples, replaced by seven-sample confirmations where available. These are the fastest and smallest **passing programs measured here**, not claims about unmeasured or excluded submissions.

| Language | Fastest passing program | Seconds | LOC | Smallest passing program | LOC | Seconds |
| --- | --- | ---: | ---: | --- | ---: | ---: |
| mech | mech-regex-final-reuse-signed | 0.490315 | 24 | mech-regex-final-reuse-signed | 24 | 0.490315 |
| c | regexredux-gcc-5 | 0.498568 | 120 | regexredux-gcc-2 | 104 | 5.985837 |
| cpp | regexredux-gpp-2 | 1.341306 | 63 | regexredux-gpp-1 | 57 | 2.163190 |
| rust | regexredux-rust-6 | 0.469501 | 77 | regexredux-rust-3 | 43 | 0.489083 |
| chapel | regexredux-chapel-3 | 1.357998 | 38 | regexredux-chapel-3 | 38 | 1.357998 |
| julia | regexredux-julia-3 | 1.764936 | 45 | regexredux-julia-1 | 40 | 2.954282 |
| go | regexredux-go-3 | 8.377778 | 87 | regexredux-go-1 | 73 | 9.162420 |
| java | regexredux-graalvmaot-6 | 2.211250 | 51 | regexredux-graalvmaot-6 | 51 | 2.211250 |
| python | regexredux-python3-2 | 0.849047 | 118 | regexredux-python3-1 | 37 | 3.957442 |
| ruby | regexredux-ruby-3 | 4.429143 | 74 | regexredux-ruby-9 | 23 | 11.376752 |
| perl | regexredux-perl-3 | 4.023780 | 64 | regexredux-perl-1 | 28 | 7.897230 |
| lua | regexredux-lua-1 | 2.044599 | 38 | regexredux-lua-1 | 38 | 2.044599 |
| luajit | regexredux-lua-1-luajit | 2.111874 | 38 | regexredux-lua-1-luajit | 38 | 2.111874 |
| javascript | regexredux-node-3 | 1.263283 | 50 | regexredux-node-4 | 32 | 1.540886 |
| php | regexredux-php-1 | 0.937362 | 61 | regexredux-php-1 | 61 | 0.937362 |

Complete pinned source LOC excludes blank/comment-only lines. Generic libraries and compiler machinery are excluded in every language. Java uses OpenJDK JIT for the last two tasks, rather than the upstream GraalVM native-image setup. LuaJIT uses unchanged Lua submissions with a compatible general library.

The broad report retains every build and validation exclusion. Its C++ pidigits #5 exclusion was resolved in the later confirmation by supplying the native-width GNU `ulong` alias on Darwin; its algorithm and source file are unchanged.

Latest confirmation: `../remaining-conformance/signed-division-confirmation.json`. Exact compiler/runtime/template snapshot is adjacent. Earlier broad snapshots additionally require `../remaining-conformance/baseline-application-templates.tar.gz`, retaining the templates omitted by their initial archive file filter.
