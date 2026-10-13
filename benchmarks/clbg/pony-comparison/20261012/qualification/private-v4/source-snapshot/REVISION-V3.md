The private-v2 metadata attempt is retained unchanged at
proposal-seals/private-v2. It ran twenty mocked methods; nineteen passed and
the retained-archive fixture failed because its expected dictionary used the
macOS /var temporary path while the implementation correctly returns resolved
/private/var paths. No proposal, bootstrap or native v2 case was launched.

The additive guarded_prepare_v3.py selects new fixture modules. The ten reuse
fixtures change only their guard import and the expected temporary dictionary
key to target.resolve(); the ten ownership fixtures change only their guard
import. All production reuse/archive/alias/native ownership and preparation
logic remains byte-for-byte identical to v2. proof_reuse_v2.py and every v1/v2
source/receipt stay unchanged. No weakened gate or benchmark change is made.

One fresh metadata attempt awaits root static review. A successful twenty-test
receipt and exact reuse/proposal digests must be reviewed before native --run.
Old twenty native cases can only be referenced for their exact four unchanged
v1 binaries; the other sixteen profiles require fresh builds and actual cases.
No native work, full timing, retry or score is launched by this draft.
