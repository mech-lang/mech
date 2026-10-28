# Selected unsigned-buffer length

The real six-position collector exhausted its unchanged computation allowance
while binding `sequence/length`. Its buffers fit the memory allowance. A temporary
operation-level work trace attributed 1,899,082,953 of 9,978,037,724 helper work
units to 242 length calls. General call binding constructed metadata and a
canonical scalar result even though the validated matrix already stores its
cardinality and the native program already owns a U64 result lane.

The minimal source requirement is:

```mech
+> sequence
sequence/length(values<[u32]>)
```

The pinned matchfinder uses the same operation in its readable-window checks;
the collector also uses it for input length. This change lowers that existing
operation through its selected, privately witnessed maintained factory. It
accepts only rank-two canonical U8/U16/U32/U64 matrices and a U64 result. It
checks the exact schema/key/shape and admission to the current managed account,
then reads the validated stored cell count. The maintained resident length
implementation supplies the checked cardinality conversion. Strings retain
their grapheme semantics and general execution path.

The callback retains the current visit, origin, complete caller coexistence,
finite call and cumulative work limits, and logical output limits. The existing
native result lane is included in capacity accounting. It allocates no binding,
canonical result owner or payload copy. Incomplete private sequence owners
continue through their existing path. A replacement catalog factory does not
receive this specialization.

This uses the existing scalar library-result protocol: program ABI 35 and typed
IR 107 are unchanged. Work policy advances from 82 to 83 because the removed
binding construction and cleanup no longer execute.

The focused tests cover all four widths, original and decoded artifacts, empty
and changing buffers, zero allocation requests for the selected call, exact
one-short work/output/capacity limits, foreign-account rejection, cleanup and
retry. An assertion-bearing function checks fresh arguments, error order,
unchanged publication and aliases after rejection, and recovery. A real
1,500,513-word input detects a physical payload copy. The existing linked
constructor-bearing qualification also executes length through an actual Object
and runtime-only consumer.

The unchanged full-buffer collector test now completes all six positions and
matches every expected tree/cache word on both artifact forms. The original
work and memory allowances and expected values are retained. This remains a
short state-update trace, not a complete compressor or compression benchmark.
