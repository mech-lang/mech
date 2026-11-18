# MechDeflate compiler execution plan

- The prior exact 65,036-byte recipe spent about 44 ms in checked-root
  retirement proofs. Refresh this interval, its enclosing root costs and
  compiler counts on the consolidated checked 65,276-byte recipe.
- Reuse immutable guard transitions within a retirement site when two
  queries start a transfer at the same program point in the same guard
  state. Keep each owner's liveness and retirement decision independent.
- Expect a complete build benefit only if reuse removes enough transitions
  to cover saved state lifetimes and cache access. Screen for at least 5 ms
  median paired source-to-final-link improvement.
- Check unchanged canonical bytecode, metadata, object and linked
  benchmark worker. Compare the ordinary proof with the prototype.
  Cached results use the same guard facts, expressions and closure state.
  Discard them before the next retirement site.
- Stop after one paired study. Rotate if the measured pool is under 20 ms,
  exact output changes or the complete build improvement is under 5 ms
  with fewer than six out of eight paired wins.
