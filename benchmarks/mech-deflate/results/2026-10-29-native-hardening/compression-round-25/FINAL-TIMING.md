# Qualified reserved append timing checkpoint

Production 4056f93118f499515048dac61d9d52558ef03aef, parent25edefcc657cc410dc4be8b71dfd50da8818041b, IR127/work93/ABI36/bytecode14. Unchanged pinned source/artifact, algorithms/tables, real buffers, source allocations, mimalloc and evaluate_into boundary.

Protected full8-byte compression: 50.493820690× to 30.494110345×Roc; 14.643208000ms to 8.843292000ms, fresh reference0.290000000ms/20000 samples. 39.608233% elapsed reduction, 1.655854856× speedup. Four alternating pairs, twelve primary/control, all observations retained. Separate453-repetition profile excluded. Exact10 raw bytes, independent inflation, capacity13, final0 bytes/nodes pass. Work 174947248798→67271224898 with unchanged route prices. No larger-input claim.

353 actual Core tests and19 focused capacity/append/image checks pass before measurement; the raw Vec reserved-view handoff and permanent zero allocation-slope assertion are included. Further component/scalar and final architecture qualification follows this immediately committed packet. No next optimization has started.

The earlier timing packet is retained as provisional because its initialized-slice-derived pointer did not establish the spare-capacity access contract. This repeat uses the reserved Vec allocation pointer directly and is the qualified comparison.
