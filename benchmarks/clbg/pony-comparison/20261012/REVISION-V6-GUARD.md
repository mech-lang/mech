The v5 metadata seal stopped before hash pins, proposal, compiler or any native
benchmark launch. Thirty-two mocked methods passed and one fixture errored:
the profile-row mock omitted its required execution_fingerprint field. The
production validator correctly rejected that incomplete mock; no gate changes.

The additive test_proof_reuse_v6.py changes only its guard import and adds the
missing mock execution_fingerprint. guarded_prepare_v6.py changes only the
selected test module. Every production source, ownership/build/probe method,
exact-prior-proof validator, failed seal receipt and prior binary is retained.
The fresh seal is proposals/private-v6.json; this is the same 33-method, 13 exact
binary/110 original-record, seven fresh-build bounded continuation scope.
