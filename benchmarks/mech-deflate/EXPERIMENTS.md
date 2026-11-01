# Compression experiments and external evidence

The authoritative current result is [native-compression-status.md](../../docs/design/native-compression-status.md).
Historical evidence through R78 is a release asset, outside the Git source tree:
[download/archive release](https://github.com/mech-lang/mech/releases/tag/evidence-native-compression-r78-20261031).
The compact [archive receipt](evidence-archive.json) pins the download, source
revision, full archive hash and internal manifest hash. The manifest records every
path, byte size, original Git blob, SHA-256 and mode. It includes failed/rejected
experiments, adverse measurements, commands, historical scripts and licenses.

The archived source snapshot is `6ab75037bc5d4ec138d735c7dcfe1c4b653d31fa`:
12,112 evidence files, 305,897,994 evidence bytes, plus 13 license files.
The 243,067,759-byte compressed bundle has SHA-256
`6b6e0c1a12cdd7bb56830958bd66db18941a4728f9b089f4e7ddaacb75265c4e`.
Removing files from the source tip does not erase historical Git objects. No
history rewrite or squash is part of this change.

## Retrieve, verify and replay

From the repository root, with Python 3 and authenticated `gh`:

```sh
python3 benchmarks/mech-deflate/experiment.py fetch --receipt benchmarks/mech-deflate/evidence-archive.json --output /tmp/mech-evidence-download
python3 benchmarks/mech-deflate/experiment.py verify /tmp/mech-evidence-download/native-compression-evidence-through-r78.tar.gz --receipt benchmarks/mech-deflate/evidence-archive.json --extract /tmp/mech-evidence-retrieved
python3 benchmarks/mech-deflate/experiment.py replay /tmp/mech-evidence-retrieved --rounds 76 77 78 --output /tmp/mech-evidence-replayed
```

Extraction requires a new directory and rejects unexpected, duplicate, unsafe or
non-file archive entries. It verifies the archive, manifest and every versioned
file against SHA-256 and original Git blob identity. Replays reconstruct archived
numbers without a compiler, compressor, executable or original scratch paths.
Historical scripts are evidence, not independent maintained harness versions.
All referenced historical paths are relative to the extracted bundle root.

Independent retrieval verified all 12,125 files and original Git blobs before
source-tip removal. Replays reconstructed **10 R76, 10 R77 and 8 R78 outputs
byte-identically**; their version/occurrence counts are in the compact receipt.

## Future experiments

Use **experiment.py** as the maintained archival/measurement entry. Put raw data
under ignored `benchmarks/mech-deflate/.experiments/` or outside the checkout.
Generated Rust/C workers and isolated source copies stay outside the checkout;
keep the exact unsafe allowlist and warning checks unchanged. An ignored alias
may retain existing experiment paths without adding code to the maintained tree.
The `measure` command accepts a JSON specification of pinned inputs, exact worker
commands, expected output/work fields and balanced sampling geometry. It uses the
existing linked-worker session: prepare both workers, disclosed warmups, alternating
blocks, untimed profile observations and checked terminal cleanup. All observations,
including failures, remain in the raw directory. Compiler builds, counter cohorts
and diagnostic instrumentation retain their exact commands and input hashes with
the packet; they must not be mixed into primary timing.

Archive and publish each completed timing cohort promptly; commit its compact
hash-addressed receipt and decision here. Do not add raw JSON/log/assembly/census
copies back to source Git. Keep source changes separate from evidence decisions.
A count improvement, instruction improvement and elapsed improvement need separate
acceptance decisions. Retain contrary cohorts and preserve every rejected attempt.

## Historical packets

Paths below name reports **inside the external bundle**. The full manifest also
covers early implementation witnesses, profiling, failed attempts and non-numbered
packets. Each report hash is SHA-256; its surrounding directory holds the complete
experiment, not only the selected report.

| Round | Report title | Report path inside bundle | SHA-256 |
| --- | --- | --- | --- |
| R01 | Complete-compressor round 01: eight literal bytes | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-01/REPORT.md` | `e6e9b4cc712a2fc9e0c08815ad2cff5d2e1a4ade3326d169966698e948be7056` |
| R02 | Round 02: packed single-assignment planning | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-02/REPORT.md` | `f26c712922ce26f96ec4e0d76f45eb290fe92b5172ca1c4ad8da51b2ddb0b8e6` |
| R03 | Round 03: schema retention summary and focused build | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-03/REPORT.md` | `312cc69fe1d6bd63af2e2607f31df2c796ed95083f7a724992f8257ca63647e2` |
| R04 | Round 04: reuse the owned turn template | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-04/REPORT.md` | `9fb781df24a28f3cad98de73cd5bc3ed1afa0192c90379fde29eecf019a0b3d9` |
| R05 | Round 05: constant-time packed payload lengths | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-05/REPORT.md` | `53e21c1399f9ac107be608d88e1fb63555b7ed672dd8d89363af6f4c3d9065b7` |
| R06 | Round 06: consuming scalar indexed stores | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-06/REPORT.md` | `0c69646a3bdf76949b92913e054eb76535c2428d567144d0ee63837567919d3e` |
| R07 | Round 07: successful native metadata precharge | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-07/REPORT.md` | `31767eab67b91a314d80ff77ee62dff8e2bd5aa7edcb4aadda3bff82299b3fa8` |
| R08 | Round 08: mixed scalar state through the existing native layout | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-08/REPORT.md` | `634148efbbdd48e038e7f10a24f3f6fc24231a610dbb7c3156998f3ab71f4f79` |
| R09 | Round 09: independent cost-table source experiment | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-09/REPORT.md` | `b172b9714812e85a88d1e0c49aacd610bfdc91f6af391d0c6b3fcfac17672f89` |
| R10 | Round 10: reuse exactly aligned managed-projection keys | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-10/REPORT.md` | `74d667c99a707f94a655f7ae59164cbb847ceca773875e76f18ddf7f72b30686` |
| R11 | Round 11: native read-only numeric leaf recurrences | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-11/REPORT.md` | `de81f06a71558dca895c33f58db63fbf3a2d9c84e3d3eda3308e734c52f132af` |
| R12 | Round 12: native private unsigned indexed-write recurrences | `benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-12/REPORT.md` | `3803b1ce88f5e53fb1b16e265c168d9d1051dc35b324d44928311f896e314162` |
| R13 | Round 13: private mixed numeric results | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-13/REPORT.md` | `741041a744ffdd952106b2207387c7870f9b99ea4dfb95c83c012886a3159292` |
| R14 | Ordinary private-frame admission review packet | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-14/CLOSURE.md` | `7b0bd524de3901033dc8088f37e9f70b2bd9f27d410497ab1f8877d7a93105eb` |
| R15 | Current-Boolean numeric version proof: review packet | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-15/CLOSURE.md` | `a647aca02c6db0193e88dfa61340d2e69358583943b407b178d23046395a22a8` |
| R16 | Ordinary aggregate metadata: completed review packet | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-16/CLOSURE.md` | `8edb0f069b0a0d70ee085288529827c317b3ccc46f31faff7433d38aac4b5347` |
| R17 | Prepared semantic descriptors: review packet | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-17/CLOSURE.md` | `94a4bf4e5c8a09dae2ffa1d0c90def3a440729c212432636b1d98439df07078b` |
| R18 | Opaque compound siblings: completed review packet | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-18/CLOSURE.md` | `65e4c0f74e6fd62a4db0e040c51b75a7545b06a975d4511d846622395892bda2` |
| R19 | Private scalar named results: completed review packet | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-19/CLOSURE.md` | `aef4f310890c28b3ee140acf970384817eddfce9a74b5aed7215b1b767278ebe` |
| R20 | Nested private-frame composition: review checkpoint | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-20/CLOSURE.md` | `3b3e631e6d011d706c422d3a34c9bcc61a260bed7b58db1f6866bcb188994812` |
| R21 | Private multiple-array numeric returns: review checkpoint | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-21/CLOSURE.md` | `0f9e38c937bee276d72b8fd0606e60f6214dd356f3a79e84e5692f77d1734d59` |
| R22 | Ordinary scalar-path admission: completed review packet | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-22/CLOSURE.md` | `6d54be1832b4da81a9b7697b9ab82c237b20d16e9b93ef16c66d982d6e6df659` |
| R23 | First-violation call budget checks: review packet | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-23/CLOSURE.md` | `54ce6dc6004d688b4b7a1f03b73739bbbe9caff04c9e51ca06658d9478c4b663` |
| R24 | Flat named record results in native numeric loops | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-24/CLOSURE.md` | `782a9a4b4d4aa27ca3e9a8aa9d04763b61e6208d2a81b313b5bf64edc189edb7` |
| R25 | Reserved-capacity unsigned appends in native numeric loops | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-25/CLOSURE.md` | `7e194b39de3bf6ce5e5c365f825200366e4c0fe2c3c8822c9941bd780a29a20a` |
| R26 | Retire dead array aliases across structural transport | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-26/CLOSURE.md` | `792174e1948cd559947416ada2b7a10bd8ef654532c2a4dff621971be4c4ca0b` |
| R27 | Scalar aggregate results in native numeric loops | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-27/CLOSURE.md` | `2e9d2a8f5678a3abc344af2d253b42855aa54d2473e0c4efa1a0c513b82a8cff` |
| R28 | Private frame scalar-read experiment: not retained | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-28/CLOSURE.md` | `a1a0616db04dd0143532f51040defbfab23a18e29dab6772bb563a39791ebc9c` |
| R29 | Shared work context: retained measured improvement | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-29/CLOSURE.md` | `5848a6a3c90e855ba86225df71ac0c98a258cc14013a813079344ffb6cbc7db3` |
| R30 | Private frame scalar read experiment: closed, not retained | `benchmarks/mech-deflate/results/2026-10-29-native-hardening/compression-round-30/CLOSURE.md` | `956450f94393bc1e8a4ee10912d49592a917159b4f166edd01b06c7701f8636a` |
| R31 | Prepared frame retirement: retained modest gain | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-31/CLOSURE.md` | `e8d7696a3ab5cfb10c67834a5d72cadba0b035c3454b30e67f8e3061f99239ba` |
| R32 | Compact call-cache storage: retained | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-32/CLOSURE.md` | `26b22c031f0a54ce42f045f76a9c831b93706afee3469b2973a1d73cb35c7a3f` |
| R33 | Bounded native parent/child composition | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-33/CLOSURE.md` | `d2409f4f1e6fc256815f87e44d7474e8876bb8b43c36ca65f30c3488ee60471b` |
| R34 | Work-meter inlining: retained 2.35% improvement | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-34/CLOSURE.md` | `6348f9673d81b53f1246b68b404997773b15bb83ca47e998e3d8f71172a02aa9` |
| R35 | Shared-debit inlining: measured regression, hint removed | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-35/CLOSURE.md` | `106994ee4a43b930ad09787138e8c237e548420076eae2a8e112c1580a79bacd` |
| R36 | Pay eligible lexical cleanup together | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-36/CLOSURE.md` | `c2ca8b81fc819ef21f9d055a6d2b223a1fc99b7a73e17078ae3fae73201f50bf` |
| R37 | Round 37: qualified layout experiment and executable profile | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-37/CLOSURE.md` | `3751aa22791a834100b39f1f346d3eb124d0124531c8c3134d33c3eab8708fdb` |
| R38 | Round 38: completed-call buffer ownership and executable scorecard | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-38/CLOSURE.md` | `06353df98a9116a3148631073520eec80c74ba0d5a4ea2c22249aebe14b93d80` |
| R39 | Native pieces around ordinary scalar child calls | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-39/CLOSURE.md` | `04f33a1dd66f784481f260601c4c29c0554e2f9792e35b3e692e3d4d52fb78b0` |
| R40 | Reject scalar aggregate grouping after matched executable measurements | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-40/CLOSURE.md` | `66fb093be0ec2c227513d25dafdaaf690921daae096b5cd60e1ea814db6c9ea6` |
| R41 | Managed projection batching: fewer instructions, neutral time | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-41/CLOSURE.md` | `373e304954157ed4eadc0671bf34ed386e1a82fdc1d4109bcfabc3c0a7a744fb` |
| R42 | Prepared record names: measured gain and executable scorecard | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-42/CLOSURE.md` | `41f12407d4ff9c4f8ebb626bcf69a089bcb62ffad5610f5de91418d873226557` |
| R43 | Move disposable private-frame owners after validation | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-43/CLOSURE.md` | `92c65d0f9547f86d4dcc5c7387a0e97e922cfd8a36edb74b5a2ac96cfd24d7cc` |
| R44 | Borrowed record-selector experiment — not retained | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-44/CLOSURE.md` | `2d705d60d8f3914b4e5482cf0d84e75bc4b8878e221bf52f476ee8c71ca5efc4` |
| R45 | Lent selector geometry — withdrawn before timing | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-45/CLOSURE.md` | `1d550c96959088fc1a599e21e1e5af69b704125386fb7e7e51d48f95fe9d226c` |
| R46 | Exact funded record-admission prefixes — retained | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-46/CLOSURE.md` | `13e649a7a267e21bfc60c7a35ae52b43ae19137307d797e4ac897e979884d920` |
| R47 | First-admission eligibility census | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-47/CLOSURE.md` | `b487bfb55e18f8d3e6171e865f1ced43c2bc21c68c592a61c7f54f59d1dcd924` |
| R48 | Unique first-admission root reuse — round 48 | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-48/CLOSURE.md` | `c619d12ffe777f8e5357898989444746a4da5b361aa5a3a9d5d4cff678c99a83` |
| R49 | Checked private-frame physical cleanup — round 49 | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-49/CLOSURE.md` | `f12d690c1a277c45a715a9c34c80fca00623612985be0f2b99c220161f09d61d` |
| R50 | Completed-child physical cleanup — round 50 | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-50/CLOSURE.md` | `4af0507ffd39092b380429be22930d6eb2ed935b2a6a39a741721e21b7d0950f` |
| R51 | Shared numeric scratch storage — round 51 | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-51/CLOSURE.md` | `f2d0157e28a26096be367ba9828383e7b75743683ce62abd2f0cafd79e8441b5` |
| R52 | Next bounded candidate: transfer dead private-frame owners | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-52/NEXT.md` | `24529408e68930809269deae2d956962507140c2a7532c01fad409907204d15a` |
| R53 | Last-use private frame owner transfer | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-53/CLOSURE.md` | `51e5a4e95593e59e9476742398319852080491314d8dd8e05d6f0adba3f2a868` |
| R54 | Remove temporary numeric matrix validation allocations | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-54/CLOSURE.md` | `691c1b04d650e4e45710b88a58be0cb0bcf09e37c0b1f3a283691ee5cac33449` |
| R55 | Exclusive registration links: retained allocation and timing improvement | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-55/CLOSURE.md` | `52411661011b0eabc7e982d3e3b56c8aaee334d0504cb6c9921379c8a9611775` |
| R56 | Private numeric publication: fewer allocations and instructions | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-56/REPORT.md` | `99e1c7fbec329cbe5fe93963a0ec148d2d1d22ca39d99341e81fff6f011121a9` |
| R57 | R57: rejected borrowed-field experiment | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-57/REPORT.md` | `4d15cb1f9165f3167e3210b893f6901d744f11400895c6c00e2daa1f284b8250` |
| R58 | R58: rejected borrowed-field experiment | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-58/REPORT.md` | `4e2ba742e56b9e3ea1c19f350af2d41a50e596296c84b07e4875fcb3d18a4be5` |
| R59 | R59: reuse the interned schema key during frame admission | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-59/REPORT.md` | `410a1847c9f534264555a53046ea5db748081ab3371bcceb6b5b66399f0b3bb9` |
| R60 | R60: static vector address generation | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-60/REPORT.md` | `cd8703143abd04266a422f24a71dafbcda4ccd432af8992832d9c637e0baf1bc` |
| R61 | R61: numeric first-change scalar transport | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-61/REPORT.md` | `168d257fb029bf9ea99900c8135d4858d40c25de2a35871c633f4af2c24f4b6d` |
| R62 | R62: captured locals-pointer locality | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-62/REPORT.md` | `5521a1109db9c910e9bc8defade00de0be0ea02e8a07f8cc998401a62442491c` |
| R63 | R63: current frame-shape guards | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-63/REPORT.md` | `0eb0c27ad6d3a41e01c7320ba53505a153e78aa423a606f761870ca55baebd87` |
| R64 | R64: first-only central projection budget checks | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-64/REPORT.md` | `5ff93ba9b0dad876f9d021632aecb9406560616679c9e733d9c3b85b80677526` |
| R65 | R65: inline the maintained checked-debit helper | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-65/REPORT.md` | `a7ae6722177ebfd05b0fe757ee00ae87632d87211973d8622927c006ae652555` |
| R66 | R66: one owner through private numeric computation | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-66/REPORT.md` | `cf7e590b09648639845d020a3fa1d2a61777180f005f4cea20c6e7018da3aad4` |
| R67 | R67: borrow the retained single-call plan | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-67/REPORT.md` | `f6e6e484a9081aef6a33cc0855a8390b113603be4f9a555ec383f77f5bd5dc6e` |
| R68 | R68: rejected descriptor-sharing experiment and instruction windows | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-68/REPORT.md` | `33cd1fd8be46d1a55ba9c241e922addd1e3921ef69e171c5d8feaf74db5344b4` |
| R69 | R69: immutable call-port array experiment — rejected | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-69/REPORT.md` | `df9c6ca761cc418154d0d487331ca2d84ca2ba806cf6ab82238afe83342ee6ad` |
| R70 | R70: borrow the owned call in observed-demand refinement | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-70/REPORT.md` | `71b373a8e621906d435f445fb4018e1b95429a22beeda0f3019075fda1ed8e41` |
| R71 | R71: forward the already checked compute total | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-71/REPORT.md` | `432eff64bc0ffaf9e897496fede41c16ac0272abe44ab687c7dfa8856146c078` |
| R72 | R72: keep the shared checked-compute commit inline | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-72/REPORT.md` | `2d090b1bbcce218a4298f6eeaf263035154be4d73d4a19360128df78f86470e4` |
| R73 | R73: inline the original ordinary debit across the Engine/runtime boundary | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-73/REPORT.md` | `b79936c564a1258394ccfe46786f10ff7e69a0caa98db00164871f1a126ed696` |
| R74 | R74: scoped current-context experiment — rejected | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-74/REPORT.md` | `491a02ed19f1aa18045ea326189cede282c31cf1a22dd19e659b0f0d24242bd5` |
| R75 | R75: scoped budget call boundaries — rejected | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-75/REPORT.md` | `9ef182f314bd642e7e4aa7d37503d2345e342fb3d3e0c1e9ae1ef587154923d1` |
| R76 | R76: keep successful debits separate from cold error transport | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-76/REPORT.md` | `2ef2b62db78f8fc69b181eff57738e3891593f6fe4fd1bd143bb8549d32739ff` |
| R77 | R77: precise private failure return — rejected | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-77/REPORT.md` | `8b4850673b44a0ca08f4f7ecd994694ed03a6bb8381fd4c6c7d02d8528536ef1` |
| R78 | R78 — adjacent admission/helper transport | `benchmarks/mech-deflate/results/2026-10-30-native-hardening/compression-round-78/REPORT.md` | `369ba9f70c9eb35e69fe9b20ac55a5b21d38e14609692c916f75daa489a776a9` |

## Current external experiments

R79 retains multi-child parent composition: **12.424308×→11.207469×** the fresh
reference, **9.794019% less elapsed**, **10.234005% fewer retired instructions**.
Requests remain6,791; code/stack grow. All adverse observations remain. The compact
[experiment receipt index](experiments.json) identifies every immutable external
packet and the four expected numerical replay hashes. The external `closure/REPORT.md`
contains exact revisions, configurations, commands/counts and failure dispositions.

```sh
python3 benchmarks/mech-deflate/experiment.py replay-round --index benchmarks/mech-deflate/experiments.json --round 79 --output /tmp/mech-r79-replay
```

This retrieves and hash-verifies every preserved version before chronological
overlay, then reconstructs the reports without compiling or compressing. Historical
R76/R77/R78 replay above remains available independently.

## R80: exclusive empty first admission

Base `d68e5f0d97c7950b11aa2a798f805f9a56cc0096`, production/optimized source
`e4d444b5fcd33a69c05837ad1cafbd4c50b9ae97`. Retained matched11.514359×→11.464732×
reference (−0.430999% elapsed),−0.489492% instructions, exactly340 fewer64-byte
requests. Object/image/metadata, source/bytecode and logical accounting are unchanged.
Four pairs /3,968 primary observations; all 16 adverse descriptive blocks retained.
All raw commands, failures, profiles, traces, controls and replay code are immutable
release packets indexed by `rounds.80` in [experiments.json](experiments.json).
Use the same `replay-round --round 80` command with a fresh output directory.

## R81: managed projections inside native parent pieces

Base `54badcdaf0a2c4094fa2127c69e25fb8bd1260ad`; production `2f9bf24de`; actual
optimized source `c29854f6f1f404658c39bcb024296b01ec80a71d`. Matched elapsed
11.077177×→10.141253× reference (−8.449124%), initial instructions −11.319245%.
Four pairs /3,968 primary observations; all blocks improve. Initial adverse cycles
and two additional predeclared hardware pairs remain separate. Actual callbacks
14,460→10,759; canonical puts 3,039→2,178, with 816 real escapes preserved.
IR134/work 96/privateABI38/MBC14; source/bytecode and algorithm pins unchanged.
Policy96 pricing golden and private current-result recipe checks are explicit.
52 backing copies disappear. Evidence, exact commands, every failed attempt and
replay hashes are external packets in `rounds.81` of [experiments.json](experiments.json).
Use `replay-round --round 81` with a fresh output directory.


## R82: immutable managed owner transport

Base `84e1375e53375c11209f9933852fac9ee98eb2b7`; production/optimized source
`4fe3ca57b17b4bcab90948f8357f8fd8fd9d11da`. Matrices of records can be forwarded
through the existing parent pieces; numeric views remain independently proved.
Elapsed alone is0.280294% adverse, instructions 0.320138% better, heap unchanged.
Callbacks10,759→10,712; all 21 moved projections escape canonically, without
eliminating an arena installation. Retained with the directly measured R83 head;
no R82 elapsed win claimed. Failed fixtures/test assertions and their corrections
are preserved. Use `replay-round --round 82` for the numerical reports.

## R83: prove byte caps from current admitted capacity

Base `3831952d49a2f2d762f33e87dadc8f29349fc9d9`; production/optimized source
`c0b5999f434650df9ff7fef6dc2c9f4c40d4fc42`. Current admitted capacity provides a
conservative sufficient bound; tight/refused/overflowing cases retain exact
footprint calculation. Current schema/account checks remain. NoLimits remains
all maxima; an untimed None-only premise was corrected from actual counters.

Direct retained R81→combined R83 elapsed10.144898×→9.969036× reference
(−1.733503%, four pairs/3,968 primary observations,4 adverse blocks retained).
Independent R82→R83 pairs improve 2.091294%; ratios/cohorts are not multiplied.
Three-way hardware cohort shows 31,824,928.5→31,050,590.5 instructions from R81
(−2.433118%); heap totals stay equal. Exact footprint walks 4,109→0, current key
checks 2,596. Tight positive byte-cap, original/decoded/restored, limit/error/retry,
scalar/frame, component/linked and architecture qualification pass. Code/stack
increase 46,120/480 bytes from retained R81, with R83 Object/image/metadata unchanged.
Use `replay-round --round 83` with a fresh output directory. Full hashes, commands,
failed/untimed attempts and separate profiles/counter cohorts stay in external
packets. See [current status](../../docs/design/native-compression-status.md).


## R84: wider zero branch through a shared resize

Base `ff4cf633c153d559a7a3f5288b34f656f94a1cfe`; production `10f5bbbe9`.
Actual code still calls outlined generic vector loops. Matched elapsed is
0.168704% lower with 28 adverse blocks; retired instructions rise 2,056.5.
Superseded by R85, not claimed as a standalone instruction win. New calibrated
coarse windows put 29.43M of31.09M diagnostic instructions in native turn+callbacks;
preparation is1.13M. Marker overhead and all failed driver/replay attempts remain.
Use `replay-round --round 84` for four reproduced reports.

## R85: distinct zero initializer exposes bulk zeroing

Base `24f1019c83c1776ff64a4fb4ef288d644f696b2e`; actual optimized/production source
`490ede99581fd233edd4c05d14f74641c9ae45c9`. A distinct zero initializer lowers all
three wider unsigned fills to bzero; nonzero, capacity, admission and work stay.
Four consistent censuses prove84 wider zero calls/6,576,864 initialized bytes.
Direct retained R83→R85 elapsed10.090325×→9.980769× reference (−1.085752%), all
four pairs improve,10 adverse blocks retained. Independent R84→R85 pairs improve
1.236842%; never multiply cohorts. Instructions are effectively flat (+202.5),
cycles improve2.281639%, heap and Object/image/metadata unchanged. All scalar/frame,
component/linked and architecture gates pass. Use `replay-round --round 85` for
four report hashes; full failures, raw observations, profiles and source pins
remain external. The [current status](../../docs/design/native-compression-status.md)
is authoritative. No source algorithm or limit changes.
