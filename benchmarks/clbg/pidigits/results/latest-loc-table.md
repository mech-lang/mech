# pidigits: latest measured endpoints

Apple M3; AC power; Low Power Mode off. Complete full workloads and stdout. Medians of three broad-sweep samples, replaced by seven-sample confirmations where available. These are the fastest and smallest **passing programs measured here**, not claims about unmeasured or excluded submissions.

| Language | Fastest passing program | Seconds | LOC | Smallest passing program | LOC | Seconds |
| --- | --- | ---: | ---: | --- | ---: | ---: |
| mech | mech-pi-final-advance-signed | 0.304389 | 42 | mech-pi-final-compact-signed | 39 | 0.396690 |
| c | pidigits-gcc-1 | 0.303447 | 45 | pidigits-gcc-1 | 45 | 0.303447 |
| cpp | pidigits-gpp-5 | 0.303652 | 84 | pidigits-gpp-4 | 50 | 0.492714 |
| rust | pidigits-rust-3 | 0.303086 | 175 | pidigits-rust-4 | 97 | 0.491783 |
| chapel | pidigits-chapel-2 | 0.318884 | 47 | pidigits-chapel-2 | 47 | 0.318884 |
| julia | pidigits-julia-3 | 0.399957 | 58 | pidigits-julia-4 | 52 | 0.902622 |
| go | pidigits-go-4 | 0.330099 | 71 | pidigits-go-3 | 65 | 0.344606 |
| java | pidigits-graalvmaot-1 | 4.165147 | 97 | pidigits-graalvmaot-1 | 97 | 4.165147 |
| python | pidigits-python3-3 | 0.440624 | 52 | pidigits-python3-4 | 43 | 4.148829 |
| ruby | pidigits-ruby-1 | 5.221127 | 69 | pidigits-ruby-1 | 69 | 5.221127 |
| perl | pidigits-perl-4 | 0.572744 | 15 | pidigits-perl-4 | 15 | 0.572744 |
| luajit | pidigits-lua-1-luajit | 1.591082 | 45 | pidigits-lua-1-luajit | 45 | 1.591082 |
| javascript | pidigits-node-2 | 5.897555 | 47 | pidigits-node-2 | 47 | 5.897555 |
| php | pidigits-php-5 | 0.412647 | 42 | pidigits-php-5 | 42 | 0.412647 |

Complete pinned source LOC excludes blank/comment-only lines. Generic libraries and compiler machinery are excluded in every language. Java uses OpenJDK JIT for the last two tasks, rather than the upstream GraalVM native-image setup. LuaJIT uses unchanged Lua submissions with a compatible general library.

The broad report retains every build and validation exclusion. Its C++ pidigits #5 exclusion was resolved in the later confirmation by supplying the native-width GNU `ulong` alias on Darwin; its algorithm and source file are unchanged.

Latest confirmation: `../remaining-conformance/signed-division-confirmation.json`. Exact compiler/runtime/template snapshot is adjacent. Earlier broad snapshots additionally require `../remaining-conformance/baseline-application-templates.tar.gz`, retaining the templates omitted by their initial archive file filter.
