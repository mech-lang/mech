# C# and Dart job11 static ownership review

This is a bounded source review and an unexecuted proposal. It does not certify counts or admit sources to charts. The frozen job11 artifact and all benchmark source bytes are preserved. SHA values below are copied from that artifact; this reviewer computed no hashes and ran no parser, lexer, test, build or benchmark.

The frozen family contains 75 source identities and 38 distinct source hashes. Its flagged subset has 35 identities, 18 distinct hashes and 39 distinct diagnostic sites. Those identities correspond to 35 current displayed registrations and 75 diagnostic registrations. Mode aliases repeat source evidence; they are not new source sites.

The approved ownership policy gives an explicit C# `using` type alias one unit. Its tuple components are type/formal structure. Complete attached generic type arguments and scalar expression method formals are structural. Pointer declarations require builtin or visible source-local type evidence, and value/formal/field shadows prevent unproved promotion. Wrapped comma declarations need full statement proof. Real callback/chain surrounds remain review gated.

## Proposed rule changes

- Parse nested generic and C# tuple type syntax with a bounded recursive grammar; protect only generic arguments immediately attached to a call or new array owner.
- Skip declaration inference in argument contexts while retaining independent assignment effects in shared core accounting.
- Recognize typed scalar arrow methods separately from declarations; assignment/executable expression method bodies remain gated.
- Own explicit C# type aliases once and record scope/lifetime; gate invalid local/class-contained, duplicate, or unproved alias forms.
- Prove C# pointers through builtin/source-local type records and conservative local/field/formal value shadow records. Namespace and arbitrary uppercase spellings are not type proof.
- Permit row trailing commas only when the complete multiline statement is a proved declaration list; retain initializer/literal wrapping.
- Treat the reviewed `default` value literals as expressions; preserve genuine switch-label gates.
- Prove an initial tuple destructuring group with balanced delimiters and an assignment excluding `=>`; tuple-return arrow methods cannot inherit destructuring ownership from a greedy match.

The draft has 46 fixture methods, 18 newly added. They include packed/expanded declarations and aliases, generic argument nesting, chained plus bare bindings, wrapped call literal rows, assignment effects in arguments, pointer lifetimes/shadows, tuple return scope boundaries, scalar methods, unresolved generic/tuple/pointer forms, and real callback surrounds. None were executed by this reviewer.

## Exact frozen sources

### binarytrees-csharpcore-3.cs

Frozen SHA `970cf62b15e5986fe68169d6ac92556b6cde985e8f80d7ddae782f869d929fb0`; baseline proposed count 56; 2 source identities and 2 current registrations.

- Line 39, offset 1339, `foreign-callback-surrounding-expression-review`: Keep the diagnostic: the written source has another callback argument or a chained then callback, with no approved surrounding-expression count proof.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### binarytrees-csharpcore-7.cs

Frozen SHA `be3604348e0b2d0d7e48978e26753d758bac1425454d482c9141d3cecee8e44d`; baseline proposed count 56; 1 source identities and 1 current registrations.

- Line 21, offset 547, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.
- Line 31, offset 904, `foreign-generic-initializer-review`: Protect a complete bounded type-argument list only when immediately attached to a generic call or new array owner; no callee-name contract.
- Line 52, offset 1731, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### fannkuchredux-csharpcore-5-arm64-faithful.cs

Frozen SHA `5e68341ac5640b9bfc1b598d3fa8a2f582ebf66471811a4a4c9f8aac1a707cb5`; baseline proposed count 150; 2 source identities and 2 current registrations.

- Line 65, offset 2423, `foreign-generic-initializer-review`: Protect a complete bounded type-argument list only when immediately attached to a generic call or new array owner; no callee-name contract.
- Line 184, offset 7785, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### knucleotide-csharpcore-1-arm64-faithful.cs

Frozen SHA `5f4cb3646bdcf031b1f9f675f8e952cfccf36f3a14d60df580032d8fd4a982dc`; baseline proposed count 158; 2 source identities and 2 current registrations.

- Line 21, offset 584, `foreign-generic-initializer-review`: Protect a complete bounded type-argument list only when immediately attached to a generic call or new array owner; no callee-name contract.
- Line 105, offset 3654, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### knucleotide-csharpcore-8-arm64-faithful.cs

Frozen SHA `888ebde7cbfb1368a0eac86960477948f29853649d1c3f4016123cf4970ab5fe`; baseline proposed count 62; 2 source identities and 2 current registrations.

- Line 20, offset 432, `foreign-generic-initializer-review`: Protect a complete bounded type-argument list only when immediately attached to a generic call or new array owner; no callee-name contract.
- Line 31, offset 744, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.
- Line 50, offset 1524, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### mandelbrot-csharpcore-7.cs

Frozen SHA `d22346944ed5d13c1dbb2d626f17f38c5ff075a23c97b31eca1bb129c6630679`; baseline proposed count 68; 2 source identities and 2 current registrations.

- Line 34, offset 1090, `foreign-generic-initializer-review`: Protect a complete bounded type-argument list only when immediately attached to a generic call or new array owner; no callee-name contract.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### nbody-csharpcore-0.cs

Frozen SHA `212627189a66c75feb27605ae77e0a1d47fcf20ea8ddc080bd4f18e97e9950a7`; baseline proposed count 88; 2 source identities and 2 current registrations.

- Line 9, offset 247, `foreign-complex-type-declaration-review`: Explicit tuple alias/component type structure or builtin/source-visible C# pointer declaration. Unknown pointer/type/value shadows remain gated.
- Line 10, offset 285, `foreign-complex-type-declaration-review`: Explicit tuple alias/component type structure or builtin/source-visible C# pointer declaration. Unknown pointer/type/value shadows remain gated.

Count decision: {'before': 3, 'after_proposal': 1, 'delta': -2, 'owner': 'using Body tuple type alias at source lines8-10; one explicit alias definition, component tuple formals structural', 'whole_source_count_status': 'requires locked qualification; anticipated alias-only delta -2'}

### nbody-csharpcore-6-arm64-faithful.cs

Frozen SHA `603be6e0c78135fc00ce3109c020dfa486df06651ef88a4f3f2a149d61de7b9a`; baseline proposed count 160; 2 source identities and 2 current registrations.

- Line 44, offset 1497, `foreign-complex-type-declaration-review`: Explicit tuple alias/component type structure or builtin/source-visible C# pointer declaration. Unknown pointer/type/value shadows remain gated.
- Line 45, offset 1548, `foreign-complex-type-declaration-review`: Explicit tuple alias/component type structure or builtin/source-visible C# pointer declaration. Unknown pointer/type/value shadows remain gated.
- Line 202, offset 8177, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.
- Line 205, offset 8296, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### pidigits-csharpcore-6-arm64-faithful.cs

Frozen SHA `a5e9734cd31f06a454ac4d2ed11bb249e8be236ea84eead9cf4677d8fe99679c`; baseline proposed count 106; 2 source identities and 2 current registrations.

- Line 16, offset 444, `foreign-pattern-switch-record-review`: These exact sites are default value literals, not switch labels. Real default/case labels retain the existing review gate.
- Line 17, offset 475, `foreign-pattern-switch-record-review`: These exact sites are default value literals, not switch labels. Real default/case labels retain the existing review gate.
- Line 18, offset 512, `foreign-pattern-switch-record-review`: These exact sites are default value literals, not switch labels. Real default/case labels retain the existing review gate.
- Line 19, offset 549, `foreign-pattern-switch-record-review`: These exact sites are default value literals, not switch labels. Real default/case labels retain the existing review gate.
- Line 20, offset 584, `foreign-pattern-switch-record-review`: These exact sites are default value literals, not switch labels. Real default/case labels retain the existing review gate.
- Line 33, offset 1098, `foreign-complex-type-declaration-review`: Explicit tuple alias/component type structure or builtin/source-visible C# pointer declaration. Unknown pointer/type/value shadows remain gated.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### regexredux-csharpcore-2-arm64-faithful.cs

Frozen SHA `4d4a9abfb13ab40f4d75269cf00e472cb73be9ea96359377d84e8920b4820466`; baseline proposed count 125; 2 source identities and 2 current registrations.

- Line 62, offset 1961, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.
- Line 63, offset 2019, `foreign-ambiguous-brace-review`: Tuple return type proves the Read function owner; its written close then proves the following Pcre type boundary.
- Line 76, offset 2355, `foreign-ambiguous-brace-review`: Tuple return type proves the Read function owner; its written close then proves the following Pcre type boundary.

Count decision: {'delta': None, 'owner': 'tuple-return Read body and subsequent Pcre type/method bodies regain explicit owner events', 'whole_source_count_status': 'requires locked qualification; do not infer full delta from two newly zero-unit opens because downstream method/control owners also change'}

### revcomp-csharpcore-3.cs

Frozen SHA `d1b214b471909300cce9f40a55d0542da654b9aaabc087ddd03d9ba579ecff9d`; baseline proposed count 100; 2 source identities and 2 current registrations.

- Line 29, offset 690, `foreign-generic-initializer-review`: Protect a complete bounded type-argument list only when immediately attached to a generic call or new array owner; no callee-name contract.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### revcomp-csharpcore-9-arm64-faithful.cs

Frozen SHA `c9c90844bf2f9ad3c2e7f09fabc01a6fcd3660694b64a1a5a01404fb61b49c4d`; baseline proposed count 248; 2 source identities and 2 current registrations.

- Line 355, offset 13991, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### spectralnorm-csharpcore-5-arm64-faithful.cs

Frozen SHA `81fad5d7b9226afe4a514ecac92d888841e9f2fb2feaeb18fc83ebc10007c170`; baseline proposed count 74; 2 source identities and 2 current registrations.

- Line 65, offset 1948, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.
- Line 85, offset 2775, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### fasta-dart-5.dart

Frozen SHA `e6fb4daa85a3266a56f10d59d92a8c11cfb04f63e90e2cc1515c0df84cef2ed5`; baseline proposed count 156; 2 source identities and 2 current registrations.

- Line 99, offset 2628, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.
- Line 119, offset 3264, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### knucleotide-dart-1.dart

Frozen SHA `d710420443c72ceb7194c1c56c12f4d1832521576d43ef32e8f9be7b1761d221`; baseline proposed count 54; 2 source identities and 2 current registrations.

- Line 26, offset 634, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### knucleotide-dart-3.dart

Frozen SHA `fd39860f284bded2df51f859e4e82c045b13f94e07406e180d3900d3230d47b9`; baseline proposed count 145; 2 source identities and 2 current registrations.

- Line 90, offset 2200, `foreign-callback-surrounding-expression-review`: Keep the diagnostic: the written source has another callback argument or a chained then callback, with no approved surrounding-expression count proof.
- Line 97, offset 2321, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### spectralnorm-dart-6.dart

Frozen SHA `41ae588b3f1630540ee0bfa4e828ad9bdd2332e005829eb006f40f4b89cea088`; baseline proposed count 135; 2 source identities and 2 current registrations.

- Line 61, offset 1434, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.
- Line 137, offset 3492, `foreign-generic-initializer-review`: Protect a complete bounded type-argument list only when immediately attached to a generic call or new array owner; no callee-name contract.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

### spectralnorm-dart-8.dart

Frozen SHA `8fd8d17cf6c2ee4fcf2cbeb846028ea39ed962f17af1abe5a27cab3edd48b811`; baseline proposed count 47; 2 source identities and 2 current registrations.

- Line 40, offset 892, `foreign-complex-declaration-review`: Prove an ordinary declarator list, a scalar expression method, or full wrapped declaration; argument contexts do not enter declaration inference.

Count decision: {'delta': 0, 'whole_source_count_status': 'reviewed diagnostic-site classification changes should preserve written units; whole-source qualification still required'}

## Required guarded qualification

The parent must freeze these two helper/test files with all shared dependencies, run the full fixture suite under `/tmp/mech-clbg-timing.lock`, and produce a new source audit. The expected direct alias correction is three written alias rows to one definition (−2) for nbody-csharpcore-0. The regex-redux tuple-return fix also restores later Pcre function/control ownership, so its full source delta must come from the new frozen recount. All other reviewed sites are classification repairs with expected site delta zero; retain full-source reconciliation before admission.
