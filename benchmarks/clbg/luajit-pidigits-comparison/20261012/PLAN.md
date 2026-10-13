# LuaJIT matching-source/library qualification

Preserve the existing LuaJIT/OpenSSL bn endpoint and every old interpreter/JIT
module. Build only new private modules using the same general GMP implementation
prepared for Lua 5.5, through `support/gmp-lua51.c`. Its separate ABI-only
compatibility header supplies absent userdata facilities, an owned dynamic
string buffer, and checked finite integral signed word arguments. LuaJIT's
existing registration/metatable API backports are used directly. The header
contains no benchmark state, loops, extraction or output.
The dynamic buffer uses Lua userdata so large decimal strings and Lua allocation
errors retain ownership; it has no fixed LUAL_BUFFERSIZE ceiling.

LuaJIT has a double-only Lua numeric representation. Scalar inputs must be
finite, integral and in the actual signed-word range; Lua 5.1's truncating
checkinteger is not used. A constructor numeric value of 2^63 takes the general
mpz_set_d branch, and decimal string constructors retain arbitrary precision.
Numeric conversion preserves ordinary GMP floating truncation and infinities;
this ABI path does not claim Lua 5.5's exact native-integer representation beyond
2^53. Generic tests must cover both sides of 2^53, the signed-word boundary,
2^63 construction, large decimal strings beyond the old fixed string buffer,
conversion overflow, quotient signs, destination aliases and storage retirement.

Use the recorded `/tmp/mech-clbg-luajit/src/luajit` runtime, its own pinned Lua
5.1 headers and existing JIT policy. Use exactly the same four original and
formatting-only normalized listed #1/#5/#6/#7 sources as the interpreter study.
Do not change their bytes or its currently queued preparation. The locked
qualification verifies original/normalized complete small/tail/adversarial and
10000-digit outputs, both actual quotient extracts through untimed tracing, and
the complete runtime/module/source/reference/pinned-harness ancestry. A sealed
separate manifest goes to root for review before any scored full launch.

The proposed study again has three fresh canonical full processes per four
listed candidates, plus three each of the unchanged current Mech fast38 and
compact35 AOT controls, eighteen total. Primary time multipliers use the fresh
fast control uniformly; fastest/fewest LOC ratios use the same 38/35 source
counts. The full candidate pools then distinguish execution-path effects while
holding general GMP library and source selection consistent. Previous OpenSSL
measurements remain explicitly historical, without being relabelled or deleted.
All runs, hashes and preparation are serialized under the common timing mutex.
There are no benchmark-specific native kernels, scaled scores or sample reuse.
