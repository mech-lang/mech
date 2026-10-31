# R62: captured locals-pointer locality

Candidate compiler: `ded89a9c0014fe66734be22ee6a3170e7ec10448`. Baseline R61 compiler: `6f412525daf0f7284d4ef6bc2bd8e21f95d22e96`. Only GPU was rebuilt; every other production library hash, feature set and profile is unchanged.

The complete eight-byte level-12 call measured **13.116782× the reference**, from **13.151673×** in the matched baseline cohort. Candidate/baseline elapsed is **0.99734703×** (-0.2653%). Reference median: 0.289000 ms.

The candidate stores the already validated program-entry locals pointer once in an invocation-owned stack slot and reloads that snapshot at each ordinary source-block entry. It does not reread the callback-visible Frame pointer. Non-program lowering is unchanged. The existing conservative visit-template verification includes the added reload; emitted machine stack and code are measured normally. This changes generated pointer transport, not compressor source, allocation behavior or logical resource charges.

| Metric | Reference | R61 baseline | R62 candidate | Change |
|---|---:|---:|---:|---:|
| Primary elapsed (ms) | 0.289000 | 3.800833 | 3.790750 | -0.2653% |
| Retired instructions | 1,364,372 | 42,257,911.5 | 42,114,188.5 | -0.3401% |
| CPU cycles | 914,161.5 | 11,911,162 | 11,796,261.5 | -0.9646% |
| Allocation requests | 95 | 8,173 | 8,173 | +0.0000% |
| Requested bytes | 9,021,923 | 98,228,396 | 98,228,396 | +0.0000% |
| Reallocations | 0 | 162 | 162 | +0.0000% |
| Peak allocator live usable bytes | 9,257,360 | 163,464,584 | 163,464,584 | +0.0000% |
| Incremental peak live usable bytes | 9,257,328 | 20,482,072 | 20,482,072 | +0.0000% |
| RSS after call | 14,761,984 | 319,340,544 | 319,127,552 | -0.0667% |
| Physical footprint after call | 14,402,432 | 228,902,656 | 228,886,272 | -0.0072% |

The separate CPU-counter cohort elapsed is 3.716229 → 3.679125 ms (-0.998445%). All 32 observations per variant are excluded from primary elapsed. This diagnostic elapsed is retained independently of the matched primary result.

Primary elapsed, CPU counters, allocation counters and sampling are separate serialized cohorts. Their percentages are not multiplied or combined into a synthetic result. Requested bytes measure allocation requests, not initialized bytes copied. Allocator peak and post-call OS footprint are distinct measures; retained compiler/catalog/image storage is included.

## Matched pairs and contrary observations

| Pair | Baseline ms | Candidate ms | Candidate/baseline |
|---|---:|---:|---:|
| 0 | 3.765645 | 3.774104 | 1.00224623× |
| 1 | 3.843542 | 3.812250 | 0.99185855× |
| 2 | 3.812813 | 3.778417 | 0.99097884× |
| 3 | 3.785208 | 3.790854 | 1.00149146× |

2/4 independently prepared pairs and 40/64 alternating blocks improve. Each pair has 16 blocks ×31 measured calls per variant: 3,968 total primary observations. Individual blocks within a prepared pair are descriptive, not independent experiments.

Counter directions that increased are retained below; an empty object means no listed median increased:

```json
{}
```

## Generated binary

| Metric | R61 baseline | R62 candidate |
|---|---:|---:|
| Generated text bytes | 13,082,688 | 11,800,572 |
| Machine entry stack bytes | 105,552 | 107,344 |
| Typed IR version | 132 | 132 |
| ABI version | 37 | 37 |

The actual linked Objects change generated text by -1,282,116 bytes and machine entry stack by +1,792 bytes. UDIV is 101 → 101. Full encoded instruction and pointer-access deltas appear in the independently pinned binary audit below. These static counts are not dynamic visits or hardware counter savings.

Pointer-role attribution correction: the older disassembly counter missed immediate updates to x16 and assigned some stack loads to a stale offset. The corrected R61 baseline contains 139,076 locals-pointer loads and 30,900 argument-lane-pointer loads, superseding the earlier 165,897 and 31,199 figures. R62 contains 39,933 and 15,445, respectively. The corrected ledger supersedes prior role attribution; whole opcode/stack-access totals and all measured retired instructions, cycles and elapsed observations are unchanged. These are static role counts, not dynamic execution counts.

The candidate Object is independently bound to its corresponding image and libraries. Equality with the baseline Object is not an acceptance requirement. Actual binary audit:

```json
{
  "scope": "Static encoded instruction and scalar stack-access comparison; not dynamic visits, retired instructions or elapsed measurements.",
  "historical_role_count_correction": "Prior audit-stack-reloads.py omitted mov x16 immediate updates, misattributing some spills to a stale pointer offset. Role-specific counts here supersede prior R61165897 locals and31199 arguments figures. Whole stack-access totals and opcode counts were unaffected.",
  "before": {
    "object.o": {
      "path": "/private/tmp/mech-first-change-spill-round61-20261031/linked-primary/after/object.o",
      "sha256": "4e222709764d92ac2b2b74e06a7237d3b901bf07cf095e1c23a7751c0dd8269d"
    },
    "metadata.json": {
      "path": "/private/tmp/mech-first-change-spill-round61-20261031/linked-primary/after/metadata.json",
      "sha256": "8c98427567f2891707a230b5d5e696a8fe0c2d39d62bd40a309ec9a905a04b4e"
    },
    "descriptor.json": {
      "path": "/private/tmp/mech-first-change-spill-round61-20261031/linked-primary/after/descriptor.json",
      "sha256": "b87db111e0978c4b045da2325598f1e79d6f47767e4e0c3d62c028e937617f59"
    }
  },
  "after": {
    "object.o": {
      "path": "/private/tmp/mech-pointer-locality-round62-20261031/linked-primary/after/object.o",
      "sha256": "4921d95fdc25eaff386a16eb3de22c0178c3171d0eba668a5bee7fabaa0f7224"
    },
    "metadata.json": {
      "path": "/private/tmp/mech-pointer-locality-round62-20261031/linked-primary/after/metadata.json",
      "sha256": "ddba92983b3f34d6e7307e10655a02cc82e23edbcdfdafa2b43e94ce1052de76"
    },
    "descriptor.json": {
      "path": "/private/tmp/mech-pointer-locality-round62-20261031/linked-primary/after/descriptor.json",
      "sha256": "8a578e3c0b3a4facbb613fc024f59f76488d00b8a7ef2974c484ce9989497702"
    }
  },
  "code_bytes": {
    "before": 13082688,
    "after": 11800572,
    "delta": -1282116
  },
  "stack_bytes": {
    "before": 105552,
    "after": 107344,
    "delta": 1792
  },
  "static_instructions": {
    "before": 3270672,
    "after": 2950143,
    "delta": -320529
  },
  "opcode_delta": {
    "add": 0,
    "adds": 0,
    "and": 0,
    "b": -4893,
    "b.eq": -2677,
    "b.hi": -55,
    "b.hs": -80,
    "b.lo": 80,
    "b.ls": 55,
    "b.ne": 2677,
    "blr": 0,
    "cbnz": 0,
    "cbz": 0,
    "clz": 0,
    "cmp": 0,
    "cset": 0,
    "eor": 0,
    "ldp": 0,
    "ldr": -119252,
    "ldrb": 0,
    "ldur": 39927,
    "lsl": 0,
    "lsr": 0,
    "madd": 0,
    "mov": -118719,
    "movk": -118538,
    "msub": 0,
    "mul": 0,
    "mvn": 0,
    "orr": 0,
    "pacibsp": 0,
    "rbit": 0,
    "retab": 0,
    "smulh": 0,
    "stp": 0,
    "str": 970,
    "strb": 0,
    "stur": -24,
    "sub": 0,
    "subs": 0,
    "udf": 0,
    "udiv": 0,
    "umulh": 0,
    "uxtb": 0
  },
  "UDIV": {
    "before": 101,
    "after": 101,
    "delta": 0
  },
  "MSUB": {
    "before": 16,
    "after": 16,
    "delta": 0
  },
  "same_typed_ir": true,
  "same_program_revision": true,
  "metadata_descriptor_agree": true,
  "stack_accesses": {
    "stack_load": {
      "before": 593954,
      "after": 514629,
      "delta": -79325
    },
    "stack_store": {
      "before": 137100,
      "after": 138046,
      "delta": 946
    },
    "indexed_stack_load": {
      "before": 438478,
      "after": 319377,
      "delta": -119101
    },
    "indexed_stack_store": {
      "before": 111582,
      "after": 112515,
      "delta": 933
    }
  },
  "pointer_roles": {
    "admit_callback": {
      "before_slots": {
        "0x19bc0": {
          "stack_load": 39938,
          "stack_store": 1,
          "indexed_stack_load": 39938,
          "indexed_stack_store": 1
        }
      },
      "after_slots": {
        "0x1a2c8": {
          "stack_load": 39938,
          "stack_store": 1,
          "indexed_stack_load": 39938,
          "indexed_stack_store": 1
        }
      },
      "loads": {
        "before": 39938,
        "after": 39938,
        "delta": 0
      },
      "indexed_loads": {
        "before": 39938,
        "after": 39938,
        "delta": 0
      }
    },
    "argument_capacity": {
      "before_slots": {
        "0x19ba0": {
          "stack_load": 8241,
          "stack_store": 1,
          "indexed_stack_load": 8241,
          "indexed_stack_store": 1
        }
      },
      "after_slots": {
        "0x1a2a8": {
          "stack_load": 8241,
          "stack_store": 1,
          "indexed_stack_load": 8241,
          "indexed_stack_store": 1
        }
      },
      "loads": {
        "before": 8241,
        "after": 8241,
        "delta": 0
      },
      "indexed_loads": {
        "before": 8241,
        "after": 8241,
        "delta": 0
      }
    },
    "arguments": {
      "before_slots": {
        "0x19ba8": {
          "stack_load": 30900,
          "stack_store": 1,
          "indexed_stack_load": 30900,
          "indexed_stack_store": 1
        }
      },
      "after_slots": {
        "0x1a2b0": {
          "stack_load": 15445,
          "stack_store": 1,
          "indexed_stack_load": 15445,
          "indexed_stack_store": 1
        }
      },
      "loads": {
        "before": 30900,
        "after": 15445,
        "delta": -15455
      },
      "indexed_loads": {
        "before": 30900,
        "after": 15445,
        "delta": -15455
      }
    },
    "context": {
      "before_slots": {
        "0x19bc8": {
          "stack_load": 55450,
          "stack_store": 1,
          "indexed_stack_load": 55450,
          "indexed_stack_store": 1
        }
      },
      "after_slots": {
        "0x1a2d0": {
          "stack_load": 55450,
          "stack_store": 1,
          "indexed_stack_load": 55450,
          "indexed_stack_store": 1
        }
      },
      "loads": {
        "before": 55450,
        "after": 55450,
        "delta": 0
      },
      "indexed_loads": {
        "before": 55450,
        "after": 55450,
        "delta": 0
      }
    },
    "frame": {
      "before_slots": {
        "0x0": {
          "stack_load": 113038,
          "stack_store": 1,
          "indexed_stack_load": 0,
          "indexed_stack_store": 0
        }
      },
      "after_slots": {
        "0x8": {
          "stack_load": 113028,
          "stack_store": 1,
          "indexed_stack_load": 0,
          "indexed_stack_store": 0
        }
      },
      "loads": {
        "before": 113038,
        "after": 113028,
        "delta": -10
      },
      "indexed_loads": {
        "before": 0,
        "after": 0,
        "delta": 0
      }
    },
    "helper": {
      "before_slots": {
        "0x19bb8": {
          "stack_load": 15518,
          "stack_store": 1,
          "indexed_stack_load": 15518,
          "indexed_stack_store": 1
        }
      },
      "after_slots": {
        "0x1a2c0": {
          "stack_load": 15518,
          "stack_store": 1,
          "indexed_stack_load": 15518,
          "indexed_stack_store": 1
        }
      },
      "loads": {
        "before": 15518,
        "after": 15518,
        "delta": 0
      },
      "indexed_loads": {
        "before": 15518,
        "after": 15518,
        "delta": 0
      }
    },
    "locals": {
      "before_slots": {
        "0x19bd0": {
          "stack_load": 139076,
          "stack_store": 2,
          "indexed_stack_load": 139076,
          "indexed_stack_store": 2
        }
      },
      "after_slots": {
        "0x0": {
          "stack_load": 39932,
          "stack_store": 1,
          "indexed_stack_load": 0,
          "indexed_stack_store": 0
        },
        "0x1a2d8": {
          "stack_load": 1,
          "stack_store": 2,
          "indexed_stack_load": 1,
          "indexed_stack_store": 2
        }
      },
      "loads": {
        "before": 139076,
        "after": 39933,
        "delta": -99143
      },
      "indexed_loads": {
        "before": 139076,
        "after": 1,
        "delta": -139075
      }
    },
    "result": {
      "before_slots": {
        "0x19bb0": {
          "stack_load": 27564,
          "stack_store": 1,
          "indexed_stack_load": 27564,
          "indexed_stack_store": 1
        }
      },
      "after_slots": {
        "0x1a2b8": {
          "stack_load": 27564,
          "stack_store": 1,
          "indexed_stack_load": 27564,
          "indexed_stack_store": 1
        }
      },
      "loads": {
        "before": 27564,
        "after": 27564,
        "delta": 0
      },
      "indexed_loads": {
        "before": 27564,
        "after": 27564,
        "delta": 0
      }
    }
  },
  "source796_action": {
    "origin": {
      "node": 0,
      "block": 93,
      "local": 0
    },
    "action": {
      "Scalar": {
        "Primitive": {
          "op": "U64Add",
          "operands": [
            856,
            79
          ],
          "output": 864
        }
      }
    },
    "terminator": {
      "Scalar": {
        "Jump": 797
      }
    },
    "iteration": 24
  },
  "source797_action": {
    "origin": {
      "node": 0,
      "block": 93,
      "local": 1
    },
    "action": {
      "LibraryCall": {
        "template": {
          "origin": {
            "node": 0,
            "block": 93,
            "local": 1
          },
          "revision": [
            33,
            244,
            154,
            195,
            242,
            73,
            114,
            71,
            237,
            46,
            47,
            190,
            103,
            52,
            70,
            64,
            57,
            45,
            132,
            161,
            159,
            120,
            13,
            135,
            114,
            173,
            220,
            213,
            5,
            211,
            150,
            193
          ],
          "inputs": [
            {
              "dimension_parameters": [
                {
                  "lifetime": "Turn",
                  "lower_bound": {
                    "Constant": 0
                  },
                  "upper_bound": null
                }
              ],
              "body": {
                "Matrix": {
                  "element": {
                    "UnsignedInteger": "W8"
                  },
                  "dimensions": [
                    {
                      "Constant": 1
                    },
                    {
                      "Parameter": 0
                    }
                  ]
                }
              }
            },
            {
              "dimension_parameters": [],
              "body": {
                "UnsignedInteger": "W64"
              }
            }
          ],
          "output": {
            "dimension_parameters": [],
            "body": {
              "UnsignedInteger": "W8"
            }
          },
          "contract": {
            "Declared": {
              "inputs": [
                {
                  "schema": 53,
                  "access": "Read",
                  "delivery": "Signal"
                },
                {
                  "schema": 6,
                  "access": "Read",
                  "delivery": "Signal"
                }
              ],
              "outputs": [
                {
                  "schema": 3,
                  "access": "Write",
                  "delivery": "Signal",
                  "construction": {
                    "FullWrite": {
                      "shape": "Declared"
                    }
                  },
                  "alias": "NoAlias",
                  "change_detection": "KernelReported"
                }
              ],
              "interaction": "Pure"
            }
          },
          "selected_key": {
            "module_path": [
              "access"
            ],
            "operation_name": "scalar"
          },
          "selected": {
            "Lexical": {
              "revision": [
                33,
                244,
                154,
                195,
                242,
                73,
                114,
                71,
                237,
                46,
                47,
                190,
                103,
                52,
                70,
                64,
                57,
                45,
                132,
                161,
                159,
                120,
                13,
                135,
                114,
                173,
                220,
                213,
                5,
                211,
                150,
                193
              ],
              "owner": 0,
              "block": 93,
              "local": 1,
              "sources": [
                {
                  "Control": {
                    "Parameter": {
                      "block": 93,
                      "ordinal": 1
                    }
                  }
                },
                {
                  "Control": {
                    "Local": {
                      "block": 93,
                      "node": 0
                    }
                  }
                }
              ],
              "leaf": {
                "selected_key": {
                  "module_path": [
                    "access"
                  ],
                  "operation_name": "scalar"
                },
                "primitive": "MatrixScalarAccess",
                "inputs": [
                  {
                    "dimension_parameters": [
                      {
                        "lifetime": "Turn",
                        "lower_bound": {
                          "Constant": 0
                        },
                        "upper_bound": null
                      }
                    ],
                    "body": {
                      "Matrix": {
                        "element": {
                          "UnsignedInteger": "W8"
                        },
                        "dimensions": [
                          {
                            "Constant": 1
                          },
                          {
                            "Parameter": 0
                          }
                        ]
                      }
                    }
                  },
                  {
                    "dimension_parameters": [],
                    "body": {
                      "UnsignedInteger": "W64"
                    }
                  }
                ],
                "output": {
                  "dimension_parameters": [],
                  "body": {
                    "UnsignedInteger": "W8"
                  }
                },
                "contract": {
                  "Declared": {
                    "inputs": [
                      {
                        "schema": 53,
                        "access": "Read",
                        "delivery": "Signal"
                      },
                      {
                        "schema": 6,
                        "access": "Read",
                        "delivery": "Signal"
                      }
                    ],
                    "outputs": [
                      {
                        "schema": 3,
                        "access": "Write",
                        "delivery": "Signal",
                        "construction": {
                          "FullWrite": {
                            "shape": "Declared"
                          }
                        },
                        "alias": "NoAlias",
                        "change_detection": "KernelReported"
                      }
                    ],
                    "interaction": "Pure"
                  }
                },
                "implementation_memory": "CanonicalFinalize"
              }
            }
          }
        },
        "operands": [
          855,
          864
        ],
        "output": 865
      }
    },
    "terminator": {
      "Scalar": {
        "Jump": 798
      }
    },
    "iteration": 24
  },
  "source_anchors": {
    "before": {
      "796": [
        "0x20368"
      ],
      "797": [
        "0x20438",
        "0x21b40",
        "0x21bf8"
      ]
    },
    "after": {
      "796": [
        "0x1b1e8"
      ],
      "797": [
        "0x1b270",
        "0x1c528",
        "0x1c5c8"
      ]
    }
  },
  "limitations": [
    "Roles inferred on successful straight-line entry before first call/branch; inspect evidence. Additional short-lived copies spilled later are not attributed by this bounded provenance pass.",
    "The verified operand/window offsets identify the same source action; source callback anchors are not per-instruction debug maps."
  ]
}
```

## Correctness and unchanged workload

The input is eight bytes `abcdefgh`; exact raw output remains `[75,76,74,78,73,77,75,207,0,0]`, with independent inflation. Output capacity remains 13 and is cleared/reused. The tree contains 262,144 U16 entries, the cache 1,500,513 U32 words, search depth is 300 and nice length 258. Input aliases, finite admission/error order, previous publication on failure and recovery remain covered. Logical work is 60,260,219,686 and comparison work 47,612; neither is a hardware instruction count. Final managed bytes and nodes remain zero.

The source and bytecode are unchanged:

```json
{
  "source_sha256": "331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9",
  "bytecode_sha256": "a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7",
  "allocator_sha256": "38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874",
  "logical_work": 60260219686,
  "comparison_work": 47612,
  "input_bytes": 8,
  "expected_output": [
    75,
    76,
    74,
    78,
    73,
    77,
    75,
    207,
    0,
    0
  ],
  "output_capacity": 13
}
```

## Qualification

Qualification status: **passed**. Exact counts come from archived inventories and executed receipts. Batches overlap; they are not a sum of unique tests.

| Receipt | Expected tests | Observed | Passed |
|---|---:|---:|---|
| `units-after/build.json` | None | None | True |
| `units-after/vector-address.json` | 1 | 1 | True |
| `units-after/first-change.json` | 1 | 1 | True |
| `units-after/preparation.json` | 2 | 2 | True |
| `units-after/runtime.json` | 328 | 328 | True |
| `units-after/work-values.json` | 32 | 32 | True |
| `qualification/component-build.json` | None | None | True |
| `qualification/path-mechanism.json` | 1 | 1 | True |
| `qualification/components.json` | 236 | 236 | True |
| `qualification/composition.json` | 8 | 8 | True |
| `ci/workflow-python.json` | 132 | 132 | True |
| `linked-acceptance/emit-build.json` | None | None | True |
| `linked-acceptance/emit.json` | None | None | True |
| `linked-acceptance/worker-build.json` | None | None | True |
| `linked-acceptance/run.json` | None | None | True |
| `snapshot-controls/parent/build.json` | None | None | True |
| `snapshot-controls/parent/run.json` | 1 | 1 | True |
| `snapshot-controls/after/build.json` | None | None | True |
| `snapshot-controls/after/run.json` | 1 | 1 | True |
| `snapshot-controls/after/ci-run.json` | 35 | 35 | True |
| `ci/results.json` | 34 | 34 | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `primary/scalar-results.json` | — | 12 | True |
| `frame-allocation/results.json` | — | 8 | True |
| `linked-acceptance/run.stdout` | — | 12 | True |

The current 329-test inventory is covered by the earlier complete 328-test batch plus the new focused test after formatting-only producer normalization. There is no claimed new 329-test execution batch; the current CI selection executes 35 tests separately.

The added callback-pointer snapshot regression executes against both original parent production and the candidate, each using the current regression source. The final fixture uses default compilation and derives its restore callback from the descriptor’s actual scalar admission frontier, so the pointer substitution spans the interior source blocks without assuming one callback per primitive. The measured compressor configuration is unchanged. The original 328-runtime and 34-CI batches remain recorded at their actual earlier test source; the later inventory has 329 runtime tests and the current CI selection executes 35. These overlapping batches are not added together as unique tests. The measured producer remains ded89a9c0014fe66734be22ee6a3170e7ec10448; the post-measurement production edit is pinned-rustfmt normalization, verified byte-for-byte before the new qualification builds.

Skipped coverage:
- complete_unchanged_knucleotide_body_uses_the_owned_emitted_map: Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.
- full remote CI: Not awaited as requested. Local exact inventory and architecture checks reported separately.
- larger compression inputs/full-scale timing: Not run; this optimization is measured on the complete pinned eight-byte level-12 call only.

Retained earlier failed attempts:
- architecture-initial/results.json: The initial architecture batch passed nine checks and failed the formatter at index8. The same architecture commands are rerun after the pinned formatter correction; final acceptance requires every rerun to pass. The initial command, output and result remain separately preserved.
- snapshot preflight attempt1 (root report; no separate raw receipt supplied): The first candidate-production comparison rejected formatting whitespace before building. It was replaced by an exact comparison against the measured producer normalized with the pinned rustfmt; no benchmark sample was discarded.
- snapshot preflight attempt2 (root report; no separate raw receipt supplied): The second candidate-production comparison rejected trailing commas before building. The final harness compares pinned-rustfmt output byte-for-byte with current production before qualification. No benchmark sample was discarded.
- snapshot-parent-run: snapshot-controls/parent/run.json: The initial fixture expected a callback at every scalar primitive. Recorded visits were [1,2,5] while primitive sites were [2,3,4]: one admission group covers sites2 through4. The final fixture uses default compilation and derives restore site5 from the descriptor admission frontier, crossing the interior source blocks. Acceptance requires the final focused test to pass on both parent and candidate. This failed execution and its source remain in the named archive; no compression timing observation is affected.
- snapshot-corrected-parent-run: snapshot-controls/parent/run.json: Disabling ScalarLoops did not change ordinary scalar admission groups, so the second fixture repeated the same incorrect callback assumption. Recorded visits were [1,2,5] while primitive sites were [2,3,4]: one admission group covers sites2 through4. The final fixture uses default compilation and derives restore site5 from the descriptor admission frontier, crossing the interior source blocks. Acceptance requires the final focused test to pass on both parent and candidate. This failed execution and its source remain in the named archive; no compression timing observation is affected.
- second snapshot attempt archive-stage collision (root report; no separate raw receipt supplied): The second fixture build succeeded, then archiving collided with a preserved prior stage name. The corrected attempt received its own archive prefix and resumed using the already successful build. Original stages remain preserved. This was not a compiler/test-build failure and no performance observation was discarded.
- Final replay after adding current-workflow and compiled-artifact receipts: The earlier 65-archive replay passed. Adding the final workflow receipt changed validation.checks[18].test_source_head from absent to 799a7807a69133decd8947afa9df4b75e011e8d6, so replay correctly rejected the stale scorecard. Recompute validation and scorecard from all final receipts before the final byte-identical replay. Both earlier report and newer receipt remain archived.

Decision: retained. Retain the measured instruction/cycle and generated-text reduction. Matched pooled elapsed is 0.265297% lower, but only two of four prepared pairs improve and geometric block ratio is adverse; elapsed speedup is not established. Allocation requests and peak allocator bytes are unchanged. Preserve the 1792-byte machine-stack increase and all contrary observations. Final publication requires passing semantic and architecture qualification.

## Replay

Run `python3 reconstruct.py /new/empty/output/path` from this evidence packet. It verifies every archived receipt version, overlays versions chronologically and reproduces scorecard, profile and validation JSON. No compiler, compressor executable or performance rerun is required.

This is a small-input complete-compression result. It does not establish larger-input throughput or a standalone minimum memory footprint. All directions and the retain/reject decision come from the observed R62 receipts and explicit decision record. No percentages from earlier rounds are combined into this result.
