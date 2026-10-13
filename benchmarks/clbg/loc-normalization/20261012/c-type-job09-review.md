# Measured C and C++ type ownership review after job09

This is an additive static source review of immutable `jobs/boundary-audit-09/candidate-audit.json`. It covers four type diagnostic kinds attached to measured candidates. Source SHA values and baseline counts come from that frozen report; no fresh hashing, lexer/normalizer imports, tests, builds, timing, source mutation, shared helper edits, or chart admission occurred in this review.

The measured scope has 34 distinct source SHA identities, 82 candidate registrations, and 79 distinct diagnostic sites. Diagnostics repeat across compiler variants: 103 typedef-owner registrations represent 36 sites; 59 possible-declaration registrations represent 31 sites; 40 instance-suffix registrations represent 10 sites; five other aggregate-suffix registrations represent two sites. Eighteen sources match at least one frozen proposal's fastest/smallest or previous-fastest/previous-smallest key. This explicit role basis differs from counting all source dictionary identities, including prepared or unmeasured candidates.

The companion JSON records every source identity, candidate registration, endpoint role, exact diagnostic offset and line, include evidence, frozen expansion, decision, and implementation category. These are proposals and source-local decisions, not whole-source certificates.

## Findings that can change a measured count

Eight binarytrees C++ sources declare `Node` with an explicit `struct Node` header. Their `Node *l,*r` fields declare two independent fields. C++ makes the named aggregate type visible before its body; the current environment only collects typedef/using aliases, so it incorrectly treats that header-owned type as unresolved. The pidigits `Mpz` type has the same gap: its header is at line 12, and `Context` declares `Mpz tmp1,tmp2` at line 63. This requires general named C++ type ownership, never an uppercase-name shortcut.

Spectralnorm's `vector<double> u(N),v(N)` and the compact Mandelbrot's `std::complex<double> Z(...),C(...)` each independently construct two variables. Their actual `<vector>`/`<complex>` includes and, for spectralnorm, `using namespace std`, provide the ownership evidence for a bounded template type and protected constructor argument groups. The declaration commas separate variables; constructor/data commas stay structural.

Six `size_t` declaration sites use real C standard headers. Two regex sources each have `size_t size,len` and `size_t ilen,clen,slen`; the latter needs three units. Two reverse-complement sources have `size_t buflen=...,len,end=0`; the uninitialized `len` adds a binding beyond the two initializer assignments. Standard header imports must establish `size_t`; it is not an unconditional compiler primitive.

The fastest regex C program's exact `pcre2.h` include establishes `PCRE2_SIZE`. Its `capacity,size` fields and two `erroroffset,pos=0` local groups each need two independent units. Static local `pcre2.h:536` defines `PCRE2_SIZE` as `size_t`; the viewed header was neither hashed nor matched to the original timing build. A narrow semantic include contract and later guarded source verification are required.

These reviewed packed declaration corrections add 22 units across distinct sources, or 38 across their measured candidate registrations. Those increments exclude later parent changes to meaningful GNU attributes or native integer aliases, and do not alter other diagnostics or certify complete source counts.

## Flags that already have the right number of units

The helper's current initializer check rejects any `<` or `>` character. That unnecessarily gates the builtin declaration `int iterations=1 << (...),c=0` and the builtin pointer declaration `char *from=w->begin,*to=w->end`. Both already count two initializer assignments, so their required count stays two. The repair must distinguish shift/member/comparison operators from actual template argument groups, without exposing template/data commas as independent bindings.

All ten anonymous-struct tails in the two nbody C++ sources are ordinary instances (`for_block`, `foreach_block`, `advance`, `energy`, `offset`). Each packed tail already contributes closing brace one plus instance one. The reverse-complement `} complement_functor{};` tail and k-nucleotide `} const cfg;` tail likewise already count two. A bounded instance suffix classifier can own these spans explicitly and remove their review gates after qualification; it must preserve the instance unit, including post-close cv qualifiers and bounded initializer braces. These twelve suffix sites need no numeric increase.

The initialized native integer groups, `int64_t` groups, and `std::string::const_iterator start=...,end=...` also already have two assignment units. They still need correct type ownership. No parameter-shadow or conditional-type diagnostic appears in the measured aggregate list reviewed here. That absence does not justify relaxing shadow/conditional gates for other sources.

## Explicit alias owners and the missing general pathways

The parent has already drafted exact intptr/uintptr/max/least/fast integer contracts and meaningful GNU attribute ownership. This review does not duplicate those changes. `typedef off_t off64_t` in the former compact C binarytrees source still needs actual POSIX/transitive import evidence; its visible source includes do not directly name `sys/types.h`. The non-endpoint C++ fannkuch source uses `int64_t` after `<iostream>`, `<vector>`, and `<algorithm>`, without a direct integer header. Preserve the import gate until the actual transitive header environment is reviewed.

Other explicit owners need bounded grammar rather than blanket acceptance:

- `using MemoryPool=std::pmr::monotonic_buffer_resource` has an exact `<memory_resource>` import and a qualified library type.
- Four binarytrees pool aliases use `boost::object_pool<Node>` after exact `boost/pool/object_pool.hpp` includes and source-owned `Node` headers.
- K-nucleotide's hash aliases use `__gnu_pbds::cc_hash_table<...>` after exact `ext/pb_ds/assoc_container.hpp` imports. A `typedef` variant wraps its type argument list across lines; a `using` variant includes `typename K::Hash`.
- The nbody tuple sources declare `H` and the `T...` pack in template formal groups, then declare `typedef H head` and `typedef tuple<T...> tail`. Template type parameters and the named tuple template need exact scoped owners.
- `using System=Body[N_BODIES]` is an array type alias over a source-owned `Body` aggregate. Its array extent is structural type syntax.
- Mandelbrot's aliases use known source class templates, `std::array`, `std::complex`, qualified size types, and a nested source type (`typename PortableBinaryBitmap::Line`). K-nucleotide's conditional alias contains `size<=16`, which is a relational operator inside a template argument rather than a nested angle opener.
- Regex aliases use `std::pair` and `std::vector`; the iterator declaration needs the specifically proven `std::string::const_iterator` member type. Arbitrary qualified members must not be declared types merely because they contain `::`.

Each reviewed explicit alias introduces one type name. That independent alias binding count does not resolve every canonical ownership decision across template headers, wrapped type arguments, or separately counted meaningful attribute groups. Preserve the frozen baseline until the generic pathway's packed/expanded fixtures and whole source replay establish those units.

The proposed implementation sequence is: explicit scoped C++ named aggregate owners; protected constructor declarators and ordinary RHS operator handling; exact standard/library import contracts; then bounded template/dependent/array type-id and template-formal ownership. Unknown bases, macros, complex declarators, unresolved type/value argument roles, missing import provenance, and shadowed names remain source-specific review gates. Whole-pool replay, including prospective compact winners, must occur under the common timing mutex before any admission.
