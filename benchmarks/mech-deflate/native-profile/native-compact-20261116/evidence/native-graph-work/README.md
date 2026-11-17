# Reuse native compiler graph work

The build keeps the fresh control-flow graph and dominator tree already
constructed by Cranelift's verifier. Identity-route discovery uses two
dominance implications to avoid a stopped path search when the result is
known. The cached search handles the remaining cases. Independent experiments
record both exact changes and the balanced source-to-link builds.
[Fresh graph verification](../verified-legalized-flowgraph/README.md)
· [Dominance queries](../dominance-stopped-reach/README.md)

| Matched Apple M1 | Source through final link | Encoder text |
| --- | ---: | ---: |
| Mech published control | 3.86114 s | 71,768 B |
| Mech with both changes | 3.83915 s | 71,768 B |
| Roc | 2.98864 s | 57,696 B |
| libdeflate | 0.47892 s | 37,080 B |

The paired median reduction is **0.500%** through the final link, and
7 of eight complete builds are faster.
Native time moves from **1.51812 to
1.49095 s**, a **1.713%**
paired reduction across eight faster native phases. Put together, the
changes satisfy the timing gate. Mech takes **1.285× Roc** and
**8.016× libdeflate** time with prebuilt compilers, fixed runtimes
and link adapters. [Predeclared plan](matched/plan.json) · [Raw matched trials](matched/timing.json.gz)

The same bytecode, complete metadata, encoder object, linked executable
and timed worker reproduce byte for byte in the campaign. The re-linked
worker passes 162 exact calls. Its two original full orders record
**81/81 Roc and 61/81 libdeflate**, with every individual win preserved.
[Artifact and memory report](report.json) · [Exact full runtime evidence](../bound-71768-runtime/README.md)
