# Source-specific review of the foreign braced families

This is an additive static review of representatives from the frozen [219-registration / 177-source-pin catalog](foreign-current-policy-review.json). It covers **C#, Dart, Zig, and Odin**: 106 displayed registrations, 69 distinct family/source-SHA pins. The excerpts below are bounded examples, not a sweep or a certificate for the whole pool.

The approved common boundary model is header 1 / executable opener 0 / independent body units / written executable close 1. Applying it requires ownership: type declarations, formal parameters, data literals, expression bodies, and executable blocks cannot all be handled by a brace/comma counter. No revised LOC totals are admitted here. Existing producer counts, reports, sources, and timing records remain unchanged.

This review ran no imports, normalization, lexers, tests, hash computations, builds, Git operations, or source-pool sweeps. Expected source SHAs are copied from the prior frozen metadata catalog; a locked pass must verify them before reuse.

## Exact representative source identities

The paths in this table are relative to `benchmarks/clbg/`. All keys and full pins are retained in the prior catalog.

| Program key | Source path | Expected source SHA-256 |
|---|---|---|
| `binarytrees:binarytrees-csharpcore-3-jit` | `remaining-languages-comparison/20261011/sources/csharp/binarytrees-csharpcore-3.cs` | `970cf62b15e5986fe68169d6ac92556b6cde985e8f80d7ddae782f869d929fb0` |
| `nbody:nbody-csharpcore-0-jit` | `remaining-languages-comparison/20261011/sources/csharp/nbody-csharpcore-0.cs` | `212627189a66c75feb27605ae77e0a1d47fcf20ea8ddc080bd4f18e97e9950a7` |
| `mandelbrot:mandelbrot-csharpcore-2-arm64-faithful-jit` | `remaining-languages-comparison/20261011/sources/csharp/mandelbrot-csharpcore-2-arm64-faithful.cs` | `9d270ef86c59f14ad11f16d731647a09e798dab7fca2a25de3086be0ce47fe20` |
| `nbody:nbody-dart-8` | `remaining-languages-comparison/20261011/sources/dart/nbody-dart-8.dart` | `51815aad44799c3a08bb1e095f635ff8b97baa632415720eef7cec8d1a104596` |
| `mandelbrot:mandelbrot-dart-1` | `remaining-languages-comparison/20261011/sources/dart/mandelbrot-dart-1.dart` | `bf85ad43630b2899b9795edc8cf12dc27ef4289104160f7b4f21873064ff7de2` |
| `mandelbrot:mandelbrot-dart-4` | `remaining-languages-comparison/20261011/sources/dart/mandelbrot-dart-4.dart` | `a5b6d719e5d76261595f260f961db5d52c9e191e227aea7c6aff21f659851299` |
| `knucleotide:knucleotide-zig-1-faithful` | `zig-odin-roc-elm-comparison/20261010/sources/knucleotide-zig-1-faithful.zig` | `b99158c914eb8117470d41043f55de3a876ccd2535f99ec69edd0703fc3b9675` |
| `binarytrees:binarytrees-zig-arena-parallel` | `zig-odin-roc-elm-comparison/20261010/sources/binarytrees-zig-arena-parallel.zig` | `e73f078c62f6b500f9d6d22e8e750be5df5cf5ca100bd5182b6d58291ab32de9` |
| `binarytrees:binarytrees-odin-arena-parallel` | `zig-odin-roc-elm-comparison/20261010/sources/binarytrees-odin-arena-parallel.odin` | `1d68787e2bb6a45c7749bfd016d69ce9c64379205017bb8a78aa206805abeb2a` |
| `nbody:nbody-odin-1-faithful` | `zig-odin-roc-elm-comparison/20261010/sources/nbody-odin-1-faithful.odin` | `0dda8916082a8db5719bd20cf0bdeff10d804eaa3e59ed00f2d4ab51ff79f865` |
| `knucleotide:knucleotide-odin-growable-maps` | `zig-odin-roc-elm-comparison/20261010/sources/knucleotide-odin-growable-maps.odin` | `fdc5eabad076f87e5b1dd6bd795487e984e80d9927819074e7bcbaaa1f7d9a98` |

## Independent declarations versus structural comma groups

| Exact source / lines | Written syntax | Required distinction |
|---|---|---|
| [C# binary trees](../../remaining-languages-comparison/20261011/sources/csharp/binarytrees-csharpcore-3.cs#L52), 52 | `private TreeNode left, right;` | Two independent uninitialized field declarators. Existing producer LOC is 62 with no expansion records, so the old assignment-only branch misses this group. |
| [Dart nbody](../../remaining-languages-comparison/20261011/sources/dart/nbody-dart-8.dart#L14), 14 / 16 / 20 | `double x, y, z, vx, vy, vz, mass;`; constructor `Body(this.x, ... this.mass);`; `var px = 0.0, py = 0.0, pz = 0.0;` | Split the seven independent field declarators and the three initialized locals. Protect constructor formal parameters. The prior tool records only line 20 (+2). |
| [Dart mandelbrot 1](../../remaining-languages-comparison/20261011/sources/dart/mandelbrot-dart-1.dart#L14), 14–17 / 27 | `const iter = 50, limit = 2.0;`; `double zr, zi, cr, ci, tr, ti;`; `final h = ..., w = h;`; `zr = zi = tr = ti = 0.0;` | Initialized and uninitialized declarators both require independent-binding handling. A chained assignment writes four independent variables, while commas inside calls/conditional expressions remain structural. Saved expansions at lines 14 (+1), 17 (+1), and 27 (+3) are useful evidence; line 15 has no expansion. |
| [Dart mandelbrot 4](../../remaining-languages-comparison/20261011/sources/dart/mandelbrot-dart-4.dart#L124), 112 / 124–125 | `var lo = 0, weighted = 0;`; `var lo = 0, hi = 0, n = 0;`; `Index(this.lo, this.hi, this.n);` | Independent local/field declarators differ from constructor formals even when both contain comma lists. |
| [Odin binary trees](../../zig-odin-roc-elm-comparison/20261010/sources/binarytrees-odin-arena-parallel.odin#L7), 7 / 24 / 36 / 41 | `Node :: struct { left, right: ^Node }`; `batch :: proc(depth, iterations: int) -> int`; `Task :: struct { depth, iterations, total: int }`; `parallel_batch :: proc(depth, iterations: int) -> int` | Grouped field declarations introduce separate fields; grouped procedure parameters belong to one formal header. The prior source has no expansion records despite these field groups. |
| [Zig binary trees](../../zig-odin-roc-elm-comparison/20261010/sources/binarytrees-zig-arena-parallel.zig#L3), 3 / 30 | `const Node = struct { left: ?*Node, right: ?*Node };`; `const Task = struct { depth: usize, iterations: usize, sum: usize = 0 };` | Split independent field declaration units within a type scope. The type binding itself, fields, and type-scope braces need explicit ownership. The prior line 30 (+1) assignment expansion does not establish the three field units. |

Comma splitting must occur only after recognizing a declaration context and protecting nested formal/call/type/literal groups. C# tuple type aliases at nbody lines 8–10 and named tuple values at lines 18–23 are concrete counterexamples to a global comma rule. C# `var (px, py, pz) = (...)` at line 45 and Odin `n, err := os.read(...)` at knucleotide line 10 are **destructuring/multi-result binding policy gates**; their independent-unit treatment must follow the common reviewed policy, not an assumption based on comma count.

## Executable headers, bodies, and written closes

| Exact source / lines | Pattern | Common boundary handling and gate |
|---|---|---|
| [C# binary trees](../../remaining-languages-comparison/20261011/sources/csharp/binarytrees-csharpcore-3.cs#L28), 28–29 / 45; 54–59; 60–68 | Named method and `for`/`if` headers; standalone `{`; standalone `}` | Associate each opener with its header. An own-line executable opener contributes zero; written executable closes retain their close events. Do not charge the `for` header's initializer/increment as independent body assignments. |
| C# binary trees, 71–72 | `if (right == null) return 1;` and `else return ...;` | Recognize separate header/body events when packed on one physical line. These are unbraced branches and have no invented written close. |
| [C# mandelbrot](../../remaining-languages-comparison/20261011/sources/csharp/mandelbrot-csharpcore-2-arm64-faithful.cs#L38), 38–46 | `do { ... } while (--i > 0);` | The trailing `while` belongs to the `do` construct. The close and trailing clause need one reviewed construct policy, rather than parsing the suffix as an unrelated loop/body. |
| [Dart mandelbrot 1](../../remaining-languages-comparison/20261011/sources/dart/mandelbrot-dart-1.dart#L22), 22 / 26 / 30 / 38 / 41–50 | C-style counted loops; unbraced `if`; `} else if (...) {` | Protect loop-header semicolons and nested condition parentheses. A combined close/branch-header/open line carries the old block's close and the next branch's header; the new opener contributes zero. |
| [Zig knucleotide](../../zig-odin-roc-elm-comparison/20261010/sources/knucleotide-zig-1-faithful.zig#L42), 42–44 / 52–62 / 73 / 76–77 | `for (s) |c| { ... }`; `while (i < frame) : (i += 1) { ... }`; `if (got == 0) break;`; `orelse return error...` | Capture parameters and the while continuation belong to the header. Explicit packed branch bodies require header/body events. `orelse return` is expression-level error flow and needs Zig-specific ownership, not an unrestricted `return` token count. |
| [Odin binary trees](../../zig-odin-roc-elm-comparison/20261010/sources/binarytrees-odin-arena-parallel.odin#L21), 21 / 70 | `if node == nil { return 0 }`; `for depth := 4; depth <= maximum; depth += 2 {` | The compact branch contains header, body, and written close events; the loop's initializer and increment remain in the header. An old expansion array of `[]` does not imply a single canonical unit. |
| [Odin knucleotide](../../zig-odin-roc-elm-comparison/20261010/sources/knucleotide-odin-growable-maps.odin#L9), 9–14 / 54–55 / 61 / 65 / 67 | Bare `for {`; packed `if ... { break/continue }`; `for width, index in widths`; `for byte, i in ...`; packed guarded assignment | Bare-loop headers, iteration capture groups, and packed bodies all need ownership. Odin source lines are statement boundaries where its grammar permits them; newline inside an expression/literal is not automatically another statement. |

`defer` appears in Zig binary trees lines 21/57/61 and Odin binary trees lines 26/66. These are ordinary source-level cleanup actions, but `defer expression` and any future `defer { body }` must be distinguished. No cleanup operation should vanish merely because it runs later.

## Callbacks and expression bodies

C# binary trees lines 33–41 contain three callback forms within one `Parallel.For` call:

```csharp
() => 0,
(i, loop, localCheck) =>
{
    return localCheck +
        (TreeNode.bottomUpTree(depth)).itemCheck();
},
localCheck => Interlocked.Add(ref check, localCheck)
```

The block callback needs its own executable owner nested inside the call-argument context. Its `},` closes the callback, while the comma remains a call separator. The expression callbacks have no written block close. Callback parameters and a wrapped return expression must not be mistaken for independent declarations. Counting function headers/expression bodies must follow the same common function policy used elsewhere; this review does not assign an extra unit to every `=>` token.

[Dart mandelbrot 4](../../remaining-languages-comparison/20261011/sources/dart/mandelbrot-dart-4.dart#L18) has both `.map((each) => assign(each))` at line 18 and `.then((segments) { ... });` at lines 21–27. The latter's `});` closes an executable callback and its enclosing invocation. `async` method headers at lines 30/36 remain method headers, and the array passed to `Isolate.spawn` at line 32 is data. A shared callback detector must carry enclosing-expression ownership rather than terminate at the first `)` or `}`.

Zig binary trees line 40 (`std.Thread.spawn(.{}, worker, .{&tasks[i]})`) and Odin binary trees line 47 (`thread.create(worker)`) pass **named function references**. They do not create a new source function/header at the call site. The function declarations at Zig line 31 and Odin line 37 remain counted in their own source locations.

## Type/data ownership and expression-control ambiguities

| Exact source / lines | Ownership evidence | Gate |
|---|---|---|
| C# binary trees, 50–74 | A class scope contains grouped fields and executable methods. | Class/type ownership cannot be treated as an inert data literal: recurse into field declarations and methods, and apply the reviewed type-scope policy explicitly. |
| Zig knucleotide, 7–18 / 24–66 | `struct` declarations contain methods, fields, and executable method blocks. | Distinguish the type declaration owner from methods nested inside it. |
| Zig knucleotide, 13 / 22 / 41; Zig binary trees, 9 / 39–40 / 59 | `&.{byte}`, `Output{}`, `Code{ .data = 0 }`, `.{ .left = null, .right = null }`, `.{} ` spawn options, tuple argument literals. | These braces are literal/initializer delimiters, not executable block closes. Named field initializers such as `.left = null` and `.right = null` do not automatically create independent statement assignments. Preserve literal wrapping under the shared rule. |
| Odin binary trees, 46; Odin nbody, 20–21 / 24–69; Odin knucleotide, 59–60 | `Task{...}`, nested `Body{[3]f64{...}, ...}`, multiline `return Sys { ... }`, fixed array literals. | Nested braces need data ownership at every level. Odin nbody lines 68 and 69 are visibly different: line 68 closes the returned array literal; line 69 closes the procedure. |
| Zig knucleotide, 28 / 53–59 / 72 | `return switch (...) { ... => ... };`, switch initializer expressions, and `catch |err| switch (err) { ... }`. | Switch arm labels and expression bodies need a specific grammar rule. `=>` is not assignment syntax. The old assignment regex nevertheless accepts it; line 28's saved +5 expansion therefore cannot certify five independent assignments. It may only be reused as a locator for renewed review. |
| Zig binary trees, 17 | `return 1 + (if (...) ... else 0) + (if (...) ... else 0);` | Expression-level `if` branches inside one return require reviewed expression ownership. Do not infer separate statement headers from every `if` keyword. |
| Odin knucleotide, 18–22 | `switch byte { case 'C', 'c': return 1 ... }` | Case-label alternatives are structural; each packed case/body needs reviewed switch-arm boundary treatment. A label comma is not a declaration split. |

The old Zig `=>` expansion is identifiable without running a lexer: [loc.py](../../zig-odin-roc-elm-comparison/20261010/loc.py) uses `(?<![=!<>])(?:[+*/-]?=(?!=)|:=)`, which does not exclude an `=` followed by `>`. This is a static explanation of existing evidence, not a recomputed count.

## Reusable common patterns and admission gates

The representatives support reusing a **shared owner/event model** for named executable functions, ordinary braced control blocks, written executable closes, grouped independent field/local declarators, callback blocks, and counted-loop headers. Each language still needs reviewed token/header recognition and declaration grammar. The owner stack must distinguish executable blocks, type declarations, literals, formal lists, call arguments, and expression-control constructs.

Before admission, review focused fixtures for at least: uninitialized fields versus constructor/formal groups; chained assignments versus destructuring/multi-result bindings; own-line opener removal; `} else ...` and `do ... while`; block versus expression callbacks; type scopes containing methods; nested record/array literals; Zig captures/while-continuations/switch/catch/orelse; and Odin grouped field/formal names, packed controls, switch cases, and newline statement boundaries.

A later locked audit must verify the source and producer pins, apply one reviewed global policy, reconcile every distinct source identity, and retain historical producer counts and timing samples. The absence of old expansion events and an old `statement-normalized` label are never admission gates. Ambiguous syntax requires explicit grammar/source review rather than fallback to a raw physical count.
