# R61: numeric first-change scalar transport

> Correction: pointer-role counts in this historical report are superseded by the [R62 audit correction](../compression-round-62/pointer-role-correction.md). The original counter missed one ARM64 immediate form. Whole-Object totals, measured hardware counters, allocations and timings remain unchanged.

Candidate compiler: `6f412525daf0f7284d4ef6bc2bd8e21f95d22e96`. Baseline R60 compiler: `aea11e1f17f05055535171fa1b0cb8b5ff2d4bcf`. Only GPU was rebuilt; every other production library hash, feature set and profile is unchanged.

The complete eight-byte level-12 call measured **13.359542× the reference**, from **13.548587×** in the matched baseline cohort. Candidate/baseline elapsed is **0.98604695×** (-1.3953%). Reference median: 0.283000 ms.

The candidate removes the redundant scalar spill in numeric_boundary materialization. Current scalar carrier fields, source index, incoming word and finite fuel are transported explicitly through the existing argument lane. Other live arithmetic values remain in SSA with normal backend call-clobber preservation. The helper has no pointer to the generated local bank, and that omitted whole-bank snapshot has no reader on this boundary. Ready-flag, origin, capacity, failure and recovery handling remains in place; ordinary bailout and successful-exit spills remain unchanged. This changes emitted stores, not the compressor algorithm, allocation behavior or logical resource charges. R60 remains the explicit baseline despite its measured elapsed tradeoff: its previous instruction reduction is not claimed as a latency reduction.

| Metric | Reference | R60 baseline | R61 candidate | Change |
|---|---:|---:|---:|---:|
| Primary elapsed (ms) | 0.283000 | 3.834250 | 3.780750 | -1.3953% |
| Retired instructions | 1,364,372 | 42,439,545.5 | 42,253,819 | -0.4376% |
| CPU cycles | 928,233.5 | 12,587,498.5 | 12,562,077.5 | -0.2020% |
| Allocation requests | 95 | 8,173 | 8,173 | +0.0000% |
| Requested bytes | 9,021,923 | 98,228,396 | 98,228,396 | +0.0000% |
| Reallocations | 0 | 162 | 162 | +0.0000% |
| Peak allocator live usable bytes | 9,257,360 | 163,464,584 | 163,464,584 | +0.0000% |
| Incremental peak live usable bytes | 9,257,328 | 20,482,072 | 20,482,072 | +0.0000% |
| RSS after call | 14,745,600 | 319,651,840 | 319,406,080 | -0.0769% |
| Physical footprint after call | 14,402,496 | 228,722,432 | 228,951,744 | +0.1003% |

The separate CPU-counter cohort elapsed is 3.937042 → 3.982542 ms (+1.155690%). All 32 observations per variant are excluded from primary elapsed. This contrary diagnostic elapsed is retained independently of the matched primary result.

Primary elapsed, CPU counters, allocation counters and sampling are separate serialized cohorts. Their percentages are not multiplied or combined into a synthetic result. Requested bytes measure allocation requests, not initialized bytes copied. Allocator peak and post-call OS footprint are distinct measures; retained compiler/catalog/image storage is included.

## Matched pairs and contrary observations

| Pair | Baseline ms | Candidate ms | Candidate/baseline |
|---|---:|---:|---:|
| 0 | 3.823333 | 3.781000 | 0.98892772× |
| 1 | 3.826709 | 3.779416 | 0.98764160× |
| 2 | 3.843438 | 3.776521 | 0.98258941× |
| 3 | 3.844270 | 3.785896 | 0.98481519× |

4/4 independently prepared pairs and 57/64 alternating blocks improve. Each pair has 16 blocks ×31 measured calls per variant: 3,968 total primary observations. Individual blocks within a prepared pair are descriptive, not independent experiments.

Counter directions that increased are retained below; an empty object means no listed median increased:

```json
{
  "physical_after": 0.10025776570965483
}
```

## Generated binary

| Metric | R60 baseline | R61 candidate |
|---|---:|---:|
| Generated text bytes | 13,806,352 | 13,082,688 |
| Machine entry stack bytes | 108,880 | 105,552 |
| Typed IR version | 132 | 132 |
| ABI version | 37 | 37 |

The actual Objects contain 36,000 fewer scalar store instructions: 7,226 fewer machine-stack stores and 28,774 fewer other stores. Static scalar stack loads decrease by 35,017. UDIV remains 101 → 101. These are encoded instruction counts, not dynamic visits or the retired-instruction saving. Generated text falls by 723,664 bytes and entry stack by 3,328 bytes.

The candidate Object is independently bound to its corresponding image and libraries. Equality with the baseline Object is not an acceptance requirement. Actual binary audit:

```json
{
  "MSUB": {
    "after": 16,
    "before": 16,
    "delta": 0
  },
  "UDIV": {
    "after": 101,
    "before": 101,
    "delta": 0
  },
  "after": {
    "descriptor.json": {
      "path": "/private/tmp/mech-first-change-spill-round61-20261031/linked-primary/after/descriptor.json",
      "sha256": "b87db111e0978c4b045da2325598f1e79d6f47767e4e0c3d62c028e937617f59"
    },
    "metadata.json": {
      "path": "/private/tmp/mech-first-change-spill-round61-20261031/linked-primary/after/metadata.json",
      "sha256": "8c98427567f2891707a230b5d5e696a8fe0c2d39d62bd40a309ec9a905a04b4e"
    },
    "object.o": {
      "path": "/private/tmp/mech-first-change-spill-round61-20261031/linked-primary/after/object.o",
      "sha256": "4e222709764d92ac2b2b74e06a7237d3b901bf07cf095e1c23a7751c0dd8269d"
    }
  },
  "before": {
    "descriptor.json": {
      "path": "/private/tmp/mech-vector-address-round60-20261031/linked-primary/after/descriptor.json",
      "sha256": "e35fb1e1fe3793cfa215aed86401c611a2838235a47b9ae87ccf598aca633114"
    },
    "metadata.json": {
      "path": "/private/tmp/mech-vector-address-round60-20261031/linked-primary/after/metadata.json",
      "sha256": "06a7bbee4ae717717aa60ccb5a9853f0ccc5ec34a86b3c854c93aa4505ad438e"
    },
    "object.o": {
      "path": "/private/tmp/mech-vector-address-round60-20261031/linked-primary/after/object.o",
      "sha256": "7a211a5fcdfbbc1ceb1ad93a305380712da0af560c30fb387a34673165f04bc5"
    }
  },
  "code_bytes": {
    "after": 13082688,
    "before": 13806352,
    "delta": -723664
  },
  "opcode_delta": {
    "add": -51,
    "adds": 0,
    "and": -422,
    "b": 1,
    "b.eq": -23,
    "b.hi": -28,
    "b.hs": -4,
    "b.lo": 4,
    "b.ls": 28,
    "b.ne": 23,
    "blr": 0,
    "cbnz": 0,
    "cbz": 0,
    "clz": 0,
    "cmp": 0,
    "cset": 0,
    "eor": 0,
    "ldp": 0,
    "ldr": -34977,
    "ldrb": 0,
    "ldur": -40,
    "lsl": -5,
    "lsr": -3,
    "madd": 0,
    "mov": -59113,
    "movk": -48608,
    "msub": 0,
    "mul": 0,
    "mvn": 0,
    "orr": -73,
    "pacibsp": 0,
    "rbit": 0,
    "retab": 0,
    "smulh": 0,
    "stp": 0,
    "str": -35975,
    "strb": 0,
    "stur": -25,
    "sub": 0,
    "subs": 0,
    "udf": 0,
    "udiv": 0,
    "umulh": 0,
    "uxtb": -1625
  },
  "same_typed_ir": true,
  "scope": "Static before/after encoded instructions; no dynamic or elapsed claims.",
  "stack_bytes": {
    "after": 105552,
    "before": 108880,
    "delta": -3328
  },
  "static_instructions": {
    "after": 3270672,
    "before": 3451588,
    "delta": -180916
  },
  "stack_accesses": {
    "scope": "Static scalar stack-access comparison; not dynamic visits or hardware counters.",
    "pointers": {
      "locals": {
        "before_slot": "0x1a8d8",
        "after_slot": "0x19bd0",
        "before_loads": 181076,
        "after_loads": 165897,
        "delta": -15179
      },
      "context": {
        "before_slot": "0x1a8d0",
        "after_slot": "0x19bc8",
        "before_loads": 55450,
        "after_loads": 55450,
        "delta": 0
      },
      "admit_callback": {
        "before_slot": "0x1a8c8",
        "after_slot": "0x19bc0",
        "before_loads": 39938,
        "after_loads": 39938,
        "delta": 0
      },
      "helper": {
        "before_slot": "0x1a8c0",
        "after_slot": "0x19bb8",
        "before_loads": 15518,
        "after_loads": 15518,
        "delta": 0
      },
      "result": {
        "before_slot": "0x1a8b8",
        "after_slot": "0x19bb0",
        "before_loads": 27564,
        "after_loads": 27564,
        "delta": 0
      },
      "arguments": {
        "before_slot": "0x1a8b0",
        "after_slot": "0x19ba8",
        "before_loads": 31255,
        "after_loads": 31199,
        "delta": -56
      },
      "argument_capacity": {
        "before_slot": "0x1a8a8",
        "after_slot": "0x19ba0",
        "before_loads": 8348,
        "after_loads": 8345,
        "delta": -3
      },
      "frame": {
        "before_slot": "0x0",
        "after_slot": "0x0",
        "before_loads": 113038,
        "after_loads": 113038,
        "delta": 0
      }
    },
    "total": {
      "stack_load": {
        "before": 628971,
        "after": 593954,
        "delta": -35017
      },
      "stack_store": {
        "before": 144326,
        "after": 137100,
        "delta": -7226
      },
      "indexed_stack_load": {
        "before": 468335,
        "after": 438478,
        "delta": -29857
      },
      "indexed_stack_store": {
        "before": 117001,
        "after": 111582,
        "delta": -5419
      }
    }
  },
  "first_change_snapshot_store_reconciliation": {
    "static_bridges": 312,
    "static_local_stores_removed": 28774,
    "regions": [
      {
        "header": 119,
        "register_words": 31,
        "first_change_boundaries": [
          160
        ],
        "removed_local_stores": 31
      },
      {
        "header": 184,
        "register_words": 86,
        "first_change_boundaries": [
          313
        ],
        "removed_local_stores": 86
      },
      {
        "header": 350,
        "register_words": 59,
        "first_change_boundaries": [
          418
        ],
        "removed_local_stores": 59
      },
      {
        "header": 724,
        "register_words": 41,
        "first_change_boundaries": [
          755
        ],
        "removed_local_stores": 41
      },
      {
        "header": 5031,
        "register_words": 44,
        "first_change_boundaries": [
          5073,
          5076
        ],
        "removed_local_stores": 88
      },
      {
        "header": 5105,
        "register_words": 39,
        "first_change_boundaries": [
          5150,
          5152
        ],
        "removed_local_stores": 78
      },
      {
        "header": 5311,
        "register_words": 39,
        "first_change_boundaries": [
          5356,
          5358
        ],
        "removed_local_stores": 78
      },
      {
        "header": 5397,
        "register_words": 44,
        "first_change_boundaries": [
          5439,
          5442
        ],
        "removed_local_stores": 88
      },
      {
        "header": 5538,
        "register_words": 53,
        "first_change_boundaries": [
          5598
        ],
        "removed_local_stores": 53
      },
      {
        "header": 5680,
        "register_words": 53,
        "first_change_boundaries": [
          5748
        ],
        "removed_local_stores": 53
      },
      {
        "header": 5759,
        "register_words": 43,
        "first_change_boundaries": [
          5795
        ],
        "removed_local_stores": 43
      },
      {
        "header": 5822,
        "register_words": 77,
        "first_change_boundaries": [
          5897,
          5907,
          5913
        ],
        "removed_local_stores": 231
      },
      {
        "header": 6024,
        "register_words": 81,
        "first_change_boundaries": [
          6130,
          6159
        ],
        "removed_local_stores": 162
      },
      {
        "header": 6260,
        "register_words": 81,
        "first_change_boundaries": [
          6366,
          6395
        ],
        "removed_local_stores": 162
      },
      {
        "header": 6482,
        "register_words": 198,
        "first_change_boundaries": [
          6672,
          6678,
          6713,
          6753
        ],
        "removed_local_stores": 792
      },
      {
        "header": 7060,
        "register_words": 51,
        "first_change_boundaries": [
          7101
        ],
        "removed_local_stores": 51
      },
      {
        "header": 7146,
        "register_words": 59,
        "first_change_boundaries": [
          7215
        ],
        "removed_local_stores": 59
      },
      {
        "header": 7226,
        "register_words": 90,
        "first_change_boundaries": [
          7286,
          7339
        ],
        "removed_local_stores": 180
      },
      {
        "header": 7407,
        "register_words": 53,
        "first_change_boundaries": [
          7475
        ],
        "removed_local_stores": 53
      },
      {
        "header": 7486,
        "register_words": 43,
        "first_change_boundaries": [
          7522
        ],
        "removed_local_stores": 43
      },
      {
        "header": 7549,
        "register_words": 77,
        "first_change_boundaries": [
          7624,
          7634,
          7640
        ],
        "removed_local_stores": 231
      },
      {
        "header": 7751,
        "register_words": 81,
        "first_change_boundaries": [
          7857,
          7886
        ],
        "removed_local_stores": 162
      },
      {
        "header": 7987,
        "register_words": 81,
        "first_change_boundaries": [
          8093,
          8122
        ],
        "removed_local_stores": 162
      },
      {
        "header": 8209,
        "register_words": 198,
        "first_change_boundaries": [
          8399,
          8405,
          8440,
          8480
        ],
        "removed_local_stores": 792
      },
      {
        "header": 8787,
        "register_words": 51,
        "first_change_boundaries": [
          8828
        ],
        "removed_local_stores": 51
      },
      {
        "header": 8873,
        "register_words": 59,
        "first_change_boundaries": [
          8942
        ],
        "removed_local_stores": 59
      },
      {
        "header": 8953,
        "register_words": 90,
        "first_change_boundaries": [
          9013,
          9066
        ],
        "removed_local_stores": 180
      },
      {
        "header": 9215,
        "register_words": 36,
        "first_change_boundaries": [
          9250
        ],
        "removed_local_stores": 36
      },
      {
        "header": 9261,
        "register_words": 36,
        "first_change_boundaries": [
          9296
        ],
        "removed_local_stores": 36
      },
      {
        "header": 9425,
        "register_words": 78,
        "first_change_boundaries": [
          9475,
          9484
        ],
        "removed_local_stores": 156
      },
      {
        "header": 9630,
        "register_words": 69,
        "first_change_boundaries": [
          9674,
          9683
        ],
        "removed_local_stores": 138
      },
      {
        "header": 9717,
        "register_words": 75,
        "first_change_boundaries": [
          9757,
          9766
        ],
        "removed_local_stores": 150
      },
      {
        "header": 9859,
        "register_words": 53,
        "first_change_boundaries": [
          9927
        ],
        "removed_local_stores": 53
      },
      {
        "header": 9938,
        "register_words": 43,
        "first_change_boundaries": [
          9974
        ],
        "removed_local_stores": 43
      },
      {
        "header": 10001,
        "register_words": 77,
        "first_change_boundaries": [
          10076,
          10086,
          10092
        ],
        "removed_local_stores": 231
      },
      {
        "header": 10203,
        "register_words": 81,
        "first_change_boundaries": [
          10309,
          10338
        ],
        "removed_local_stores": 162
      },
      {
        "header": 10439,
        "register_words": 81,
        "first_change_boundaries": [
          10545,
          10574
        ],
        "removed_local_stores": 162
      },
      {
        "header": 10661,
        "register_words": 198,
        "first_change_boundaries": [
          10851,
          10857,
          10892,
          10932
        ],
        "removed_local_stores": 792
      },
      {
        "header": 11239,
        "register_words": 51,
        "first_change_boundaries": [
          11280
        ],
        "removed_local_stores": 51
      },
      {
        "header": 11325,
        "register_words": 59,
        "first_change_boundaries": [
          11394
        ],
        "removed_local_stores": 59
      },
      {
        "header": 11405,
        "register_words": 90,
        "first_change_boundaries": [
          11465,
          11518
        ],
        "removed_local_stores": 180
      },
      {
        "header": 12130,
        "register_words": 43,
        "first_change_boundaries": [
          12178
        ],
        "removed_local_stores": 43
      },
      {
        "header": 12189,
        "register_words": 74,
        "first_change_boundaries": [
          12283
        ],
        "removed_local_stores": 74
      },
      {
        "header": 12294,
        "register_words": 58,
        "first_change_boundaries": [
          12365
        ],
        "removed_local_stores": 58
      },
      {
        "header": 12388,
        "register_words": 23,
        "first_change_boundaries": [
          12402
        ],
        "removed_local_stores": 23
      },
      {
        "header": 12868,
        "register_words": 53,
        "first_change_boundaries": [
          12928
        ],
        "removed_local_stores": 53
      },
      {
        "header": 13284,
        "register_words": 26,
        "first_change_boundaries": [
          13301
        ],
        "removed_local_stores": 26
      },
      {
        "header": 13312,
        "register_words": 58,
        "first_change_boundaries": [
          13375
        ],
        "removed_local_stores": 58
      },
      {
        "header": 13386,
        "register_words": 41,
        "first_change_boundaries": [
          13426
        ],
        "removed_local_stores": 41
      },
      {
        "header": 13537,
        "register_words": 26,
        "first_change_boundaries": [
          13554
        ],
        "removed_local_stores": 26
      },
      {
        "header": 13565,
        "register_words": 58,
        "first_change_boundaries": [
          13628
        ],
        "removed_local_stores": 58
      },
      {
        "header": 13639,
        "register_words": 41,
        "first_change_boundaries": [
          13679
        ],
        "removed_local_stores": 41
      },
      {
        "header": 13701,
        "register_words": 70,
        "first_change_boundaries": [
          13785
        ],
        "removed_local_stores": 70
      },
      {
        "header": 13796,
        "register_words": 102,
        "first_change_boundaries": [
          13926
        ],
        "removed_local_stores": 102
      },
      {
        "header": 13937,
        "register_words": 85,
        "first_change_boundaries": [
          14044
        ],
        "removed_local_stores": 85
      },
      {
        "header": 14066,
        "register_words": 70,
        "first_change_boundaries": [
          14150
        ],
        "removed_local_stores": 70
      },
      {
        "header": 14161,
        "register_words": 102,
        "first_change_boundaries": [
          14291
        ],
        "removed_local_stores": 102
      },
      {
        "header": 14302,
        "register_words": 85,
        "first_change_boundaries": [
          14409
        ],
        "removed_local_stores": 85
      },
      {
        "header": 14430,
        "register_words": 70,
        "first_change_boundaries": [
          14514
        ],
        "removed_local_stores": 70
      },
      {
        "header": 14525,
        "register_words": 102,
        "first_change_boundaries": [
          14655
        ],
        "removed_local_stores": 102
      },
      {
        "header": 14666,
        "register_words": 85,
        "first_change_boundaries": [
          14773
        ],
        "removed_local_stores": 85
      },
      {
        "header": 14788,
        "register_words": 70,
        "first_change_boundaries": [
          14872
        ],
        "removed_local_stores": 70
      },
      {
        "header": 14883,
        "register_words": 102,
        "first_change_boundaries": [
          15013
        ],
        "removed_local_stores": 102
      },
      {
        "header": 15024,
        "register_words": 85,
        "first_change_boundaries": [
          15131
        ],
        "removed_local_stores": 85
      },
      {
        "header": 15265,
        "register_words": 23,
        "first_change_boundaries": [
          15279
        ],
        "removed_local_stores": 23
      },
      {
        "header": 15737,
        "register_words": 143,
        "first_change_boundaries": [
          15815,
          15867,
          15916
        ],
        "removed_local_stores": 429
      },
      {
        "header": 16010,
        "register_words": 53,
        "first_change_boundaries": [
          16078
        ],
        "removed_local_stores": 53
      },
      {
        "header": 16089,
        "register_words": 43,
        "first_change_boundaries": [
          16125
        ],
        "removed_local_stores": 43
      },
      {
        "header": 16152,
        "register_words": 77,
        "first_change_boundaries": [
          16227,
          16237,
          16243
        ],
        "removed_local_stores": 231
      },
      {
        "header": 16354,
        "register_words": 81,
        "first_change_boundaries": [
          16460,
          16489
        ],
        "removed_local_stores": 162
      },
      {
        "header": 16590,
        "register_words": 81,
        "first_change_boundaries": [
          16696,
          16725
        ],
        "removed_local_stores": 162
      },
      {
        "header": 16812,
        "register_words": 198,
        "first_change_boundaries": [
          17002,
          17008,
          17043,
          17083
        ],
        "removed_local_stores": 792
      },
      {
        "header": 17390,
        "register_words": 51,
        "first_change_boundaries": [
          17431
        ],
        "removed_local_stores": 51
      },
      {
        "header": 17476,
        "register_words": 59,
        "first_change_boundaries": [
          17545
        ],
        "removed_local_stores": 59
      },
      {
        "header": 17556,
        "register_words": 90,
        "first_change_boundaries": [
          17616,
          17669
        ],
        "removed_local_stores": 180
      },
      {
        "header": 17737,
        "register_words": 53,
        "first_change_boundaries": [
          17805
        ],
        "removed_local_stores": 53
      },
      {
        "header": 17816,
        "register_words": 43,
        "first_change_boundaries": [
          17852
        ],
        "removed_local_stores": 43
      },
      {
        "header": 17879,
        "register_words": 77,
        "first_change_boundaries": [
          17954,
          17964,
          17970
        ],
        "removed_local_stores": 231
      },
      {
        "header": 18081,
        "register_words": 81,
        "first_change_boundaries": [
          18187,
          18216
        ],
        "removed_local_stores": 162
      },
      {
        "header": 18317,
        "register_words": 81,
        "first_change_boundaries": [
          18423,
          18452
        ],
        "removed_local_stores": 162
      },
      {
        "header": 18539,
        "register_words": 198,
        "first_change_boundaries": [
          18729,
          18735,
          18770,
          18810
        ],
        "removed_local_stores": 792
      },
      {
        "header": 19117,
        "register_words": 51,
        "first_change_boundaries": [
          19158
        ],
        "removed_local_stores": 51
      },
      {
        "header": 19203,
        "register_words": 59,
        "first_change_boundaries": [
          19272
        ],
        "removed_local_stores": 59
      },
      {
        "header": 19283,
        "register_words": 90,
        "first_change_boundaries": [
          19343,
          19396
        ],
        "removed_local_stores": 180
      },
      {
        "header": 19545,
        "register_words": 36,
        "first_change_boundaries": [
          19580
        ],
        "removed_local_stores": 36
      },
      {
        "header": 19591,
        "register_words": 36,
        "first_change_boundaries": [
          19626
        ],
        "removed_local_stores": 36
      },
      {
        "header": 19755,
        "register_words": 78,
        "first_change_boundaries": [
          19805,
          19814
        ],
        "removed_local_stores": 156
      },
      {
        "header": 19960,
        "register_words": 69,
        "first_change_boundaries": [
          20004,
          20013
        ],
        "removed_local_stores": 138
      },
      {
        "header": 20047,
        "register_words": 75,
        "first_change_boundaries": [
          20087,
          20096
        ],
        "removed_local_stores": 150
      },
      {
        "header": 20189,
        "register_words": 53,
        "first_change_boundaries": [
          20257
        ],
        "removed_local_stores": 53
      },
      {
        "header": 20268,
        "register_words": 43,
        "first_change_boundaries": [
          20304
        ],
        "removed_local_stores": 43
      },
      {
        "header": 20331,
        "register_words": 77,
        "first_change_boundaries": [
          20406,
          20416,
          20422
        ],
        "removed_local_stores": 231
      },
      {
        "header": 20533,
        "register_words": 81,
        "first_change_boundaries": [
          20639,
          20668
        ],
        "removed_local_stores": 162
      },
      {
        "header": 20769,
        "register_words": 81,
        "first_change_boundaries": [
          20875,
          20904
        ],
        "removed_local_stores": 162
      },
      {
        "header": 20991,
        "register_words": 198,
        "first_change_boundaries": [
          21181,
          21187,
          21222,
          21262
        ],
        "removed_local_stores": 792
      },
      {
        "header": 21569,
        "register_words": 51,
        "first_change_boundaries": [
          21610
        ],
        "removed_local_stores": 51
      },
      {
        "header": 21655,
        "register_words": 59,
        "first_change_boundaries": [
          21724
        ],
        "removed_local_stores": 59
      },
      {
        "header": 21735,
        "register_words": 90,
        "first_change_boundaries": [
          21795,
          21848
        ],
        "removed_local_stores": 180
      },
      {
        "header": 22442,
        "register_words": 43,
        "first_change_boundaries": [
          22490
        ],
        "removed_local_stores": 43
      },
      {
        "header": 22501,
        "register_words": 74,
        "first_change_boundaries": [
          22595
        ],
        "removed_local_stores": 74
      },
      {
        "header": 22606,
        "register_words": 58,
        "first_change_boundaries": [
          22677
        ],
        "removed_local_stores": 58
      },
      {
        "header": 22773,
        "register_words": 53,
        "first_change_boundaries": [
          22833
        ],
        "removed_local_stores": 53
      },
      {
        "header": 22915,
        "register_words": 53,
        "first_change_boundaries": [
          22983
        ],
        "removed_local_stores": 53
      },
      {
        "header": 22994,
        "register_words": 43,
        "first_change_boundaries": [
          23030
        ],
        "removed_local_stores": 43
      },
      {
        "header": 23057,
        "register_words": 77,
        "first_change_boundaries": [
          23132,
          23142,
          23148
        ],
        "removed_local_stores": 231
      },
      {
        "header": 23259,
        "register_words": 81,
        "first_change_boundaries": [
          23365,
          23394
        ],
        "removed_local_stores": 162
      },
      {
        "header": 23495,
        "register_words": 81,
        "first_change_boundaries": [
          23601,
          23630
        ],
        "removed_local_stores": 162
      },
      {
        "header": 23717,
        "register_words": 198,
        "first_change_boundaries": [
          23907,
          23913,
          23948,
          23988
        ],
        "removed_local_stores": 792
      },
      {
        "header": 24295,
        "register_words": 51,
        "first_change_boundaries": [
          24336
        ],
        "removed_local_stores": 51
      },
      {
        "header": 24381,
        "register_words": 59,
        "first_change_boundaries": [
          24450
        ],
        "removed_local_stores": 59
      },
      {
        "header": 24461,
        "register_words": 90,
        "first_change_boundaries": [
          24521,
          24574
        ],
        "removed_local_stores": 180
      },
      {
        "header": 24642,
        "register_words": 53,
        "first_change_boundaries": [
          24710
        ],
        "removed_local_stores": 53
      },
      {
        "header": 24721,
        "register_words": 43,
        "first_change_boundaries": [
          24757
        ],
        "removed_local_stores": 43
      },
      {
        "header": 24784,
        "register_words": 77,
        "first_change_boundaries": [
          24859,
          24869,
          24875
        ],
        "removed_local_stores": 231
      },
      {
        "header": 24986,
        "register_words": 81,
        "first_change_boundaries": [
          25092,
          25121
        ],
        "removed_local_stores": 162
      },
      {
        "header": 25222,
        "register_words": 81,
        "first_change_boundaries": [
          25328,
          25357
        ],
        "removed_local_stores": 162
      },
      {
        "header": 25444,
        "register_words": 198,
        "first_change_boundaries": [
          25634,
          25640,
          25675,
          25715
        ],
        "removed_local_stores": 792
      },
      {
        "header": 26022,
        "register_words": 51,
        "first_change_boundaries": [
          26063
        ],
        "removed_local_stores": 51
      },
      {
        "header": 26108,
        "register_words": 59,
        "first_change_boundaries": [
          26177
        ],
        "removed_local_stores": 59
      },
      {
        "header": 26188,
        "register_words": 90,
        "first_change_boundaries": [
          26248,
          26301
        ],
        "removed_local_stores": 180
      },
      {
        "header": 26339,
        "register_words": 43,
        "first_change_boundaries": [
          26387
        ],
        "removed_local_stores": 43
      },
      {
        "header": 26398,
        "register_words": 74,
        "first_change_boundaries": [
          26492
        ],
        "removed_local_stores": 74
      },
      {
        "header": 26503,
        "register_words": 58,
        "first_change_boundaries": [
          26574
        ],
        "removed_local_stores": 58
      },
      {
        "header": 26606,
        "register_words": 43,
        "first_change_boundaries": [
          26654
        ],
        "removed_local_stores": 43
      },
      {
        "header": 26665,
        "register_words": 74,
        "first_change_boundaries": [
          26759
        ],
        "removed_local_stores": 74
      },
      {
        "header": 26770,
        "register_words": 58,
        "first_change_boundaries": [
          26841
        ],
        "removed_local_stores": 58
      },
      {
        "header": 26864,
        "register_words": 23,
        "first_change_boundaries": [
          26878
        ],
        "removed_local_stores": 23
      },
      {
        "header": 27336,
        "register_words": 143,
        "first_change_boundaries": [
          27414,
          27466,
          27515
        ],
        "removed_local_stores": 429
      },
      {
        "header": 27609,
        "register_words": 53,
        "first_change_boundaries": [
          27677
        ],
        "removed_local_stores": 53
      },
      {
        "header": 27688,
        "register_words": 43,
        "first_change_boundaries": [
          27724
        ],
        "removed_local_stores": 43
      },
      {
        "header": 27751,
        "register_words": 77,
        "first_change_boundaries": [
          27826,
          27836,
          27842
        ],
        "removed_local_stores": 231
      },
      {
        "header": 27953,
        "register_words": 81,
        "first_change_boundaries": [
          28059,
          28088
        ],
        "removed_local_stores": 162
      },
      {
        "header": 28189,
        "register_words": 81,
        "first_change_boundaries": [
          28295,
          28324
        ],
        "removed_local_stores": 162
      },
      {
        "header": 28411,
        "register_words": 198,
        "first_change_boundaries": [
          28601,
          28607,
          28642,
          28682
        ],
        "removed_local_stores": 792
      },
      {
        "header": 28989,
        "register_words": 51,
        "first_change_boundaries": [
          29030
        ],
        "removed_local_stores": 51
      },
      {
        "header": 29075,
        "register_words": 59,
        "first_change_boundaries": [
          29144
        ],
        "removed_local_stores": 59
      },
      {
        "header": 29155,
        "register_words": 90,
        "first_change_boundaries": [
          29215,
          29268
        ],
        "removed_local_stores": 180
      },
      {
        "header": 29336,
        "register_words": 53,
        "first_change_boundaries": [
          29404
        ],
        "removed_local_stores": 53
      },
      {
        "header": 29415,
        "register_words": 43,
        "first_change_boundaries": [
          29451
        ],
        "removed_local_stores": 43
      },
      {
        "header": 29478,
        "register_words": 77,
        "first_change_boundaries": [
          29553,
          29563,
          29569
        ],
        "removed_local_stores": 231
      },
      {
        "header": 29680,
        "register_words": 81,
        "first_change_boundaries": [
          29786,
          29815
        ],
        "removed_local_stores": 162
      },
      {
        "header": 29916,
        "register_words": 81,
        "first_change_boundaries": [
          30022,
          30051
        ],
        "removed_local_stores": 162
      },
      {
        "header": 30138,
        "register_words": 198,
        "first_change_boundaries": [
          30328,
          30334,
          30369,
          30409
        ],
        "removed_local_stores": 792
      },
      {
        "header": 30716,
        "register_words": 51,
        "first_change_boundaries": [
          30757
        ],
        "removed_local_stores": 51
      },
      {
        "header": 30802,
        "register_words": 59,
        "first_change_boundaries": [
          30871
        ],
        "removed_local_stores": 59
      },
      {
        "header": 30882,
        "register_words": 90,
        "first_change_boundaries": [
          30942,
          30995
        ],
        "removed_local_stores": 180
      },
      {
        "header": 31055,
        "register_words": 23,
        "first_change_boundaries": [
          31069
        ],
        "removed_local_stores": 23
      },
      {
        "header": 31527,
        "register_words": 143,
        "first_change_boundaries": [
          31605,
          31657,
          31706
        ],
        "removed_local_stores": 429
      },
      {
        "header": 31800,
        "register_words": 53,
        "first_change_boundaries": [
          31868
        ],
        "removed_local_stores": 53
      },
      {
        "header": 31879,
        "register_words": 43,
        "first_change_boundaries": [
          31915
        ],
        "removed_local_stores": 43
      },
      {
        "header": 31942,
        "register_words": 77,
        "first_change_boundaries": [
          32017,
          32027,
          32033
        ],
        "removed_local_stores": 231
      },
      {
        "header": 32144,
        "register_words": 81,
        "first_change_boundaries": [
          32250,
          32279
        ],
        "removed_local_stores": 162
      },
      {
        "header": 32380,
        "register_words": 81,
        "first_change_boundaries": [
          32486,
          32515
        ],
        "removed_local_stores": 162
      },
      {
        "header": 32602,
        "register_words": 198,
        "first_change_boundaries": [
          32792,
          32798,
          32833,
          32873
        ],
        "removed_local_stores": 792
      },
      {
        "header": 33180,
        "register_words": 51,
        "first_change_boundaries": [
          33221
        ],
        "removed_local_stores": 51
      },
      {
        "header": 33266,
        "register_words": 59,
        "first_change_boundaries": [
          33335
        ],
        "removed_local_stores": 59
      },
      {
        "header": 33346,
        "register_words": 90,
        "first_change_boundaries": [
          33406,
          33459
        ],
        "removed_local_stores": 180
      },
      {
        "header": 33527,
        "register_words": 53,
        "first_change_boundaries": [
          33595
        ],
        "removed_local_stores": 53
      },
      {
        "header": 33606,
        "register_words": 43,
        "first_change_boundaries": [
          33642
        ],
        "removed_local_stores": 43
      },
      {
        "header": 33669,
        "register_words": 77,
        "first_change_boundaries": [
          33744,
          33754,
          33760
        ],
        "removed_local_stores": 231
      },
      {
        "header": 33871,
        "register_words": 81,
        "first_change_boundaries": [
          33977,
          34006
        ],
        "removed_local_stores": 162
      },
      {
        "header": 34107,
        "register_words": 81,
        "first_change_boundaries": [
          34213,
          34242
        ],
        "removed_local_stores": 162
      },
      {
        "header": 34329,
        "register_words": 198,
        "first_change_boundaries": [
          34519,
          34525,
          34560,
          34600
        ],
        "removed_local_stores": 792
      },
      {
        "header": 34907,
        "register_words": 51,
        "first_change_boundaries": [
          34948
        ],
        "removed_local_stores": 51
      },
      {
        "header": 34993,
        "register_words": 59,
        "first_change_boundaries": [
          35062
        ],
        "removed_local_stores": 59
      },
      {
        "header": 35073,
        "register_words": 90,
        "first_change_boundaries": [
          35133,
          35186
        ],
        "removed_local_stores": 180
      },
      {
        "header": 35224,
        "register_words": 43,
        "first_change_boundaries": [
          35272
        ],
        "removed_local_stores": 43
      },
      {
        "header": 35283,
        "register_words": 74,
        "first_change_boundaries": [
          35377
        ],
        "removed_local_stores": 74
      },
      {
        "header": 35388,
        "register_words": 58,
        "first_change_boundaries": [
          35459
        ],
        "removed_local_stores": 58
      },
      {
        "header": 35609,
        "register_words": 36,
        "first_change_boundaries": [
          35644
        ],
        "removed_local_stores": 36
      },
      {
        "header": 35655,
        "register_words": 36,
        "first_change_boundaries": [
          35690
        ],
        "removed_local_stores": 36
      },
      {
        "header": 35819,
        "register_words": 78,
        "first_change_boundaries": [
          35869,
          35878
        ],
        "removed_local_stores": 156
      },
      {
        "header": 36024,
        "register_words": 69,
        "first_change_boundaries": [
          36068,
          36077
        ],
        "removed_local_stores": 138
      },
      {
        "header": 36111,
        "register_words": 75,
        "first_change_boundaries": [
          36151,
          36160
        ],
        "removed_local_stores": 150
      },
      {
        "header": 36253,
        "register_words": 53,
        "first_change_boundaries": [
          36321
        ],
        "removed_local_stores": 53
      },
      {
        "header": 36332,
        "register_words": 43,
        "first_change_boundaries": [
          36368
        ],
        "removed_local_stores": 43
      },
      {
        "header": 36395,
        "register_words": 77,
        "first_change_boundaries": [
          36470,
          36480,
          36486
        ],
        "removed_local_stores": 231
      },
      {
        "header": 36597,
        "register_words": 81,
        "first_change_boundaries": [
          36703,
          36732
        ],
        "removed_local_stores": 162
      },
      {
        "header": 36833,
        "register_words": 81,
        "first_change_boundaries": [
          36939,
          36968
        ],
        "removed_local_stores": 162
      },
      {
        "header": 37055,
        "register_words": 198,
        "first_change_boundaries": [
          37245,
          37251,
          37286,
          37326
        ],
        "removed_local_stores": 792
      },
      {
        "header": 37633,
        "register_words": 51,
        "first_change_boundaries": [
          37674
        ],
        "removed_local_stores": 51
      },
      {
        "header": 37719,
        "register_words": 59,
        "first_change_boundaries": [
          37788
        ],
        "removed_local_stores": 59
      },
      {
        "header": 37799,
        "register_words": 90,
        "first_change_boundaries": [
          37859,
          37912
        ],
        "removed_local_stores": 180
      },
      {
        "header": 39196,
        "register_words": 111,
        "first_change_boundaries": [
          39311,
          39334
        ],
        "removed_local_stores": 222
      },
      {
        "header": 39702,
        "register_words": 55,
        "first_change_boundaries": [
          39762
        ],
        "removed_local_stores": 55
      },
      {
        "header": 39801,
        "register_words": 55,
        "first_change_boundaries": [
          39861
        ],
        "removed_local_stores": 55
      },
      {
        "header": 39900,
        "register_words": 89,
        "first_change_boundaries": [
          40008
        ],
        "removed_local_stores": 89
      },
      {
        "header": 40825,
        "register_words": 36,
        "first_change_boundaries": [
          40863
        ],
        "removed_local_stores": 36
      }
    ]
  }
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
| `units-parent/build.json` | None | None | True |
| `units-parent/vector-address.json` | 1 | 1 | True |
| `units-after/build.json` | None | None | True |
| `units-after/vector-address.json` | 1 | 1 | True |
| `units-parent/first-change.json` | 1 | 1 | True |
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
| `ci/results.json` | 34 | 34 | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture-audit-fixed/result.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `architecture/results.json` | — | — | True |
| `primary/scalar-results.json` | — | 12 | True |
| `frame-allocation/results.json` | — | 8 | True |
| `linked-acceptance/run.stdout` | — | 12 | True |

Skipped coverage:
- complete_unchanged_knucleotide_body_uses_the_owned_emitted_map: Existing runtime selection exclusion retained; unrelated complete knucleotide body is not requalified here.
- full remote CI: Not awaited as requested. Local exact inventory and architecture checks reported separately.
- larger compression inputs/full-scale timing: Not run; this optimization is measured on the complete pinned eight-byte level-12 call only.

Retained earlier failed attempts:
- units-parent-initial/build.json: The new test had ambiguous boxed shape inference. The explicit shape type correction compiled successfully; unchanged parent production then passed the new exact regression before the spill removal was applied. The original failed command and stderr remain archived.
- architecture/results.json: The unsafe-boundary scan found unsafe text in duplicated external regalloc.rs dependency snapshots copied as review evidence. Their public duplicate source files were removed, with evidence retained in compressed archives. No allowlist or production source was changed. The exact same unsafe-boundary command was rerun; its observed result is required for acceptance.

Decision: retained. All required local qualification passes after correcting the review-evidence layout without changing the unsafe allowlist or production source. The matched complete-call primary elapsed improves 1.395305%, retired instructions improve 0.437626%, and cycles improve 0.201954%; exact output, logical work, error/retry behavior and allocation counts are preserved. The separate CPU-counter diagnostic elapsed (+1.155690%) and post-call physical footprint (+0.100258%) remain adverse and explicitly retained. The later R56-to-R61 confirmation is a separate cohort and does not contribute to this decision.

## Replay

Run `python3 reconstruct.py /new/empty/output/path` from this evidence packet. It verifies every archived receipt version, overlays versions chronologically and reproduces scorecard, profile and validation JSON. No compiler, compressor executable or performance rerun is required.

This is a small-input complete-compression result. It does not establish larger-input throughput or a standalone minimum memory footprint. All directions and the retain/reject decision come from the observed R61 receipts and explicit decision record. No percentages from earlier rounds are combined into this result.


## Separate final confirmation: R56 to R61

A fresh elapsed-only comparison of the accepted starting R56 executable and final R61 executable measured **13.656178× → 13.553403×** the shared reference (3.8783545 → 3.8491665 ms; -0.752587%). 3/4 prepared pairs and 49/64 blocks improve. This is a separate 3,968-observation cohort with its own 0.284000 ms reference; no earlier percentage is multiplied into it.

| Pair | R56 ms | R61 ms | Change |
|---|---:|---:|---:|
| 0 | 3.8158960 | 3.7779795 | -0.993646% |
| 1 | 3.8915410 | 3.8744170 | -0.440031% |
| 2 | 3.8465830 | 3.7973750 | -1.279265% |
| 3 | 4.0638540 | 4.1339165 | +1.724041% |

The adverse fourth pair is retained. This confirmation establishes no additional instruction, cycle, allocation or peak-memory result; those remain the separate per-change cohorts. `confirm.py` recomputes every timed row and all 64 block medians, verifies the two frozen worker bindings and reproduces `confirmation-scorecard.json` as the fifth replay output.
