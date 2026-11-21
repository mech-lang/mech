# Literal allocation setup through the caller frame

- The current 64,652-byte root redirects 26 allocation calls through an
  ordinary four-operand heap-frame stub. The latest independent literal
  setup pass increases the complete object by 48 bytes on this recipe.
- Combine the existing heap and literal setup in one private stub. Keep
  capacity dynamic and restore only proved stride, alignment and zeroing
  literals before tail-branching to the original allocation entry.
- A one-argument call should remove repeated argument moves and constant
  materialization. Price the complete emitted object and fresh source
  through final link after the ordinary and specialized recipes compile.
- Match the original five-word import signature, unchanged heap argument,
  two-word result signature and direct I64 constants. Require equal
  supported stride/alignment and preserve zeroing's original nonzero
  interpretation. Check the resolved caller frame slot, exact output,
  injected failures with recovery, aliases, cleanup and peak stack.
- Preserve the original qualified worker. A peer loss leads to a measured
  recovery attempt on this smaller branch. Promotion requires both full
  orders with 81 Roc and at least 62 libdeflate wins, and source reproduction.
