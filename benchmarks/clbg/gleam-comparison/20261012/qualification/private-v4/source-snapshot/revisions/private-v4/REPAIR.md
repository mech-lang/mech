Private-v3 started all20 reviewed metadata fixture methods. Nineteen passed; its
grouping-source lineage fixture failed with FileNotFoundError because the
validator read private-v3/sources although every profile points to the preserved
private-v2 sources. No proposal, compiler or native benchmark was launched.

V4 changes that one validation root to the actual retained private-v2/sources.
The five corrected sources, six grouping edits, profiles, historical fingerprint
projection and all runtime/artifact evidence remain unchanged. The same20 IDs
must pass. The existing grouping fixture now checks that retained root, absence
of copied sources and the actual v3 failed receipt/process/raw-log prerequisite.
Both earlier failures remain preserved and newly captured evidence is labelled.

An exclusive terminal journal keeps actual metadata outcomes and their pins on
success or failure, including a late failure after a saved proposal. Every hash,
fixture and journal stays under the common timing mutex. Native work remains a
separate reviewed digest stage:19 fresh builds,152 fresh cases plus4 old cases.

Root/peer source review precedes one metadata seal:

    python3 -B revisions/private-v4/guarded_prepare.py --seal --proposal-file proposals/private-v4.json

This packet is source-only and unqueued. No timings or chart entries change.
