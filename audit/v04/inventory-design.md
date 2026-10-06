# Inventory replenishment: execution and evidence

The retained program begins with 100 units. Each turn computes `stock + arrivals - demand` in Mech. Four source integrity constraints require nonnegative arrivals, demand and stock, and a maximum stock of 1,000,000. A rejected prepared turn preserves the accepted output, publication epoch and state hash. Subsequent inputs reach the same resident instance.

The native executable and browser instance use the exact source in `evidence/inventory/inventory.mec`. The browser package includes a crate-local copy, exported by `inventorySource()`. The page compares this source with its served fixture and records SHA-256. Native and browser evidence compare source bytes and nine expected results.

## Public boundary

The opt-in `i64_publication` feature adds `I64PublicationSession(source, inputNamesJson)`. The constructor supplies signed i64 input schemas to `CanonicalSourceFrontend`, validates strict syntax, compiles an artifact, encodes and decodes bytecode-v1, and activates `ReactiveInstance`. `update(inputJson)` converts named decimal strings to canonical `ValueDraft` scalars, rebinds them to the compiled ports, and invokes `prepare_turn_values` followed by `publish`. `snapshot()` reports copied outputs, their schemas, the publication epoch and the state hash. Decimal text preserves exact i64 values at the JavaScript boundary. The session owns the resident instance for its lifetime; page teardown frees it.

The adapter contains source-independent transport and inspection. A second test compiles a retained accumulator from 7, with external port `change`, and checks updates +3 and −4 against outputs 10 and 6. The inventory calculation and four constraints remain in the retained source fixture.

## Independent checks

The literal nine-turn fixture expects stocks 105, 105, 90, 0, 5, 5, 5, 5 and 12. Attempts 2, 6, 7 and 8 reject. The fixture covers overdraw, negative arrivals, negative demand, capacity overflow and recovery. The initialization turn supplies zero arrivals and demand. Each accepted turn checks the recurrence; each rejected turn compares output, epoch and state hash before and after. A separate ledger checks conservation: accepted stock equals 100 plus accepted arrivals minus accepted demand.

The page admits interactive quantities from −1,000,000 through 1,000,000. This stated domain keeps intermediate signed arithmetic within i64. The UI validates decimal syntax before submission. Adapter tests separately reject missing port names, a decimal exceeding i64, and a JSON number supplied where exact decimal text is required. Three fresh instances verify independent initialization.

The browser harness runs the nine-turn corpus, then uses the visible controls for arrivals/demand (8,3), (0,18) and (2,4), checking acceptance, rejection and recovery. It verifies malformed form input, receipt selection, conservation and restart, and records screenshots. Every executable record contains the exact loaded WASM hash and source modification manifest.

## Measured semantic boundary

The initial source used an interval-constrained stock directly in arithmetic. The canonical frontend rejected that operand with `source-semantics/non-numeric-arithmetic-kind`. The interval contract excludes arithmetic capability. The preserved native rejection establishes this boundary. The final source uses ordinary signed integers and integrity constraints, both supported through the generic source/runtime path. Source revisions, native logs and bytecode are retained under `evidence/inventory/`.

## Recorded outcomes

The final native workload, both native adapter tests, the nine browser fixture turns, the alternate-source accumulator, admission failures and visible-control checks passed. Native and browser source SHA-256 is `b20d99a51835470f52fc70f462acde751f0458fb8f406ce4f732b4ada63e7a3a`. The shared final WASM SHA-256 is `f833f55e74cbbd2887ba6431d0ef0fb1aa433e53ef374e8e79c355512b9d0194`; its size is 208,709,347 bytes. The manifest identifies the broad browser feature configuration and all source modifications.

## Reproduction

Build and record the shared browser artifact with the feature command in `README.md`, serve the repository, then run `python3 audit/v04/check_inventory_browser.py`. Native reproduction is in `evidence/inventory/README.md`. Browser results and screenshots are `evidence/inventory-browser.json`, `evidence/inventory-browser.png` and `evidence/inventory-receipts.png`.
