# Bounded C# and Dart job12 fixture repair

The frozen `jobs/boundary-audit-12/fixtures.log` records 611 methods, 605 passes and six failures before source auditing. This reviewer owns five C#/Dart failures; the parent owns the separate Ada failure. The frozen job and its executed receipt remain unchanged. This repair is a text-only proposal, with no parser imports, normalization, tests, hashes, builds or job execution by the reviewer.

The two existing explicit callback fixtures expected three units and obtained four. Source-position metadata was appended to the context tuple, replacing the immediate grammar owner observed through `context[-1]` by the shared callback suffix helper. Prepending metadata preserves the actual argument/data frame at the end and restores the existing zero-unit closing punctuation proof. Header, body and written close counts stay unchanged.

The nested generic constructor and generic method fixtures received a stale generic gate. The suspect-pattern scan also matched inner `List<...>` or `Vector128<...>` type lists because its lookahead could reach an outer closing angle followed by a call. A nested opening is now accepted only when it lies inside a complete proved attached type-argument span. The bounded recursive type parser has already proved the whole span, so this retains the unknown-syntax gate rather than admitting an arbitrary inner name.

The wrapped `Use<int,string>(` first row received another stale gate despite its complete attached type list and argument context. The declaration fallback now invokes the same unproved-only predicate instead of flagging every generic spelling. Literal argument wrapping remains three written rows.

Expected counts and existing gate assertions are preserved. A nested malformed generic contrast was added inside an existing method, so `test_foreign_braced.py` remains at 46 methods. Unknown generic operators, pointer/type/value ownership, tuple-return arrow ambiguity and actual callback surrounds stay gated. Qualification requires a new immutable snapshot and guarded full fixture run by the parent under the common timing mutex.
