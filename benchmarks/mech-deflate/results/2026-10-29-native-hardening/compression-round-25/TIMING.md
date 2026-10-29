# Reserved append timing checkpoint

Production 03c48bc0fe50b6f486b89050d7d1c361ff9cf274, parent25edefcc657cc410dc4be8b71dfd50da8818041b, IR127/work93/ABI36/bytecode14. Unchanged pinned source/artifact, algorithms/tables, real buffers, source allocations, mimalloc and evaluate_into boundary.

Protected full8-byte compression: 49.394059524× to 30.075113946×Roc; 14.521853500ms to 8.842083500ms, fresh reference0.294000000ms/20000 samples. 39.111881% elapsed reduction, 1.642356522× speedup. Four alternating pairs, twelve primary/control, all observations retained. Separate453-repetition profile excluded. Exact10 raw bytes, independent inflation, capacity13, final0 bytes/nodes pass. Work 174947248798→67271224898 with unchanged route prices. No larger-input claim.

352 actual Core tests and19 focused capacity/append/image checks pass before measurement; all12 architecture checks pass. Further component/scalar qualification follows this immediately committed packet. No next optimization has started.

This first experiment is **provisional and superseded for qualification**. Review of the first-change pointer handoff finds that it derives an append address through an initialized slice. Reserved writes need the Vec allocation view directly, without a subsequent slice borrow while that view is live. All raw samples remain unchanged. Correct the Core/runtime view boundary, add the actual Core all-width/empty-prefix raw-write regression, rebuild, and repeat against the original frozen25edef baseline and a fresh common reference before claiming a qualified gain. This is a correctness repair, not an additional tuning round. See [Vec::as_mut_ptr guarantees](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.as_mut_ptr).
