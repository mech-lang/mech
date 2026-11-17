# Checked byte-read outlining

The exact 71,768-byte reference reproduces its object, bytecode and complete
metadata with the root IR dump enabled. In that dump the largest repeated
single-entry shape contains a length check and defaulted word read at 51
sites. Another shape contains 15 byte reads with a general matrix index
calculation. The existing outliner proves checked boundaries, captures and
result transport for each selected instance.
[Root dump receipt](dump-receipt.json) · [Repeated region census](regions.json)

| Source-generated static variant | Encoder text | Checked-root text extent | Root frame |
| --- | ---: | ---: | ---: |
| Published reference | 71,768 B | 51,264 B | 3,728 B |
| Reference without root tail pad | 71,752 B | 51,248 B | 3,728 B |
| Shared defaulted word reads | 72,544 B | 51,872 B | 3,792 B |
| Shared general byte reads | **71,420 B** | 50,752 B | 3,776 B |
| Shared general index calculation | 71,752 B | 51,184 B | 3,632 B |

The general byte-read outline saves 332 bytes against the unpadded control.
Its root extent has the published root's address residue modulo 64 without
the old tail pad. The two linked workers match all 162 exact output and
resource checks and six checked failures. Each has a measured peak stack of
5,280 bytes. The encoder text is 1.238× Roc and 1.926× libdeflate.
[Static results](static-summary.json) · [Contract checks](contract-checks/result.json)

## Runtime

| Order | Roc, reference and outline | libdeflate, reference and outline |
| --- | ---: | ---: |
| Focused forward | 15/15 | 11/15 |
| Focused reverse | 15/15 | 11/15 |
| Complete forward | 81/81 | 61/81 |
| Complete reverse | 81/81 | 60/81 |

The outline keeps every individual control win. Both workers cross parity
against libdeflate on `prefix-periodic-1024` in the complete reverse order.
The reference takes 1.00208× libdeflate and the outline takes 1.00417×.
Both complete orders contain 96 balanced blocks per case and 982,656 timed
calls. Policy v2 reports **Unresolved** for the shared 61-win floor miss.
[Focused plan and replay](focus/plan.json) · [Complete plan](full/plan.json)
· [Complete replay](full/replay.json.gz) · [Offline replay](replay-runtime.py)

## Reproduction and build time

The checked [source recipe patch](shared-byte-recipe.patch) adds the
`shared-byte-read-layout` rebuild option. Apply it to a scratch checkout to
reproduce the candidate from the same compiler and Mech source. The verified
recipe emits object `1ba5b62ab2f54deaa2b34ea1d97a635458a09dd981e471789e6bd94fedd15460`
and the complete matched metadata.
[Recipe receipt](optional-recipe.json) · [Rebuild receipt](optional-reproduction.json)

| Same compiler and fixed deployment support | Source through final link | Source through strip |
| --- | ---: | ---: |
| Mech reference | 2.81116 s | 2.82187 s |
| Mech byte-read outline | 2.81196 s | 2.82248 s |
| Roc | 2.99060 s | 3.00108 s |
| libdeflate | 0.48163 s | 0.49202 s |

The paired build time is 1.0013× the reference, 0.940× Roc and 5.841×
libdeflate in this separate eight-order comparison. The stripped file
is 292,576 bytes with its canonical output name.
[Build plan](source-through-link-plan.json) · [Matched observations](source-through-link-observations.json.gz)
