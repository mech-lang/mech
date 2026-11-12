# Inline terminal output transfer

One inline attribute is the only source difference from the exact terminal-slot worker. All three workers link the same generated R355 Object and use the same allocator and complete-call timer.

| Input | Inline / R355 | Inline / terminal only | Faster matched blocks |
| --- | ---: | ---: | ---: |
| empty | 0.960034× | 0.952664× | 23/24 |
| one-stored | 0.952377× | 0.955515× | 23/24 |
| seven-stored-prefix | 1.009947× | 0.981830× | 21/24 |
| eight-literals | 1.010702× | 0.999248× | 10/24 |
| prefix-natural-262144 | 1.000914× | 1.001527× | 12/24 |

Linked code eliminates the out-of-line transfer call and expands the static wrapper from 77 to 122 instructions. The separate terminal-slot branch still avoids growing the free list when it transfers the last owner. Thirteen focused tests passed.

All 115,272 complete calls retain exact streams, independent inflation, output capacity, aliases and cleanup. The archive preserves the zero timer observations and the six payload counter distributions. The full 81-case comparison remains a separate acceptance step.

[Raw timing observations and offline replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/terminal-owner-inline-b15f4243e9ff.tar.gz).
