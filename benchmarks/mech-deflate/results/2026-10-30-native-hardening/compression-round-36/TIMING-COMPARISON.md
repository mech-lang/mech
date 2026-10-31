# Distinguish latency changes from changes in the reference measurement

All rows are complete eight-byte compression. The fixed ruler is the saved
round-32 Roc median, **0.300 ms**. It makes elapsed-time progress comparable;
it does not claim that Roc always takes that long. Fresh session references
remain preserved separately. Raw receipts are unchanged.

| Saved result | Mech ms | Session reference ms | Session multiple | Fixed 0.300 ms multiple | Status |
| --- | ---: | ---: | ---: | ---: | --- |
| Round 32 aggregate, after | 5.0813960 | 0.300 | 16.937987× | 16.937987× | Retained |
| Round 33 first pair, after | 4.9922705 | 0.288 | 17.334273× | 16.640902× | Provisional, superseded |
| Round 33 confirmed aggregate, after | 5.0451670 | 0.279 | 18.083036× | 16.817223× | Retained |
| Round 34 aggregate, after | 4.9173750 | 0.279 | 17.625000× | 16.391250× | Retained |
| Round 35 aggregate, before | 4.9211040 | 0.285 | 17.267032× | 16.403680× | Retained baseline |
| Round 35 aggregate, candidate | 5.0422500 | 0.285 | 17.692105× | 16.807500× | Rejected and removed |
| Round 36 aggregate, before | 4.9301665 | 0.287 | 17.178280× | 16.433888× | Paired baseline |
| Round 36 aggregate, after | 4.6873125 | 0.287 | 16.332099× | 15.624375× | Retained |

The saved 17.33× entry is 4.9922705 / 0.288. The preceding saved aggregate is
16.94×, 5.081396 / 0.300; an exact 16.99× receipt was not found. Between those
entries Mech takes **1.75% less time** while the reference takes **4% less time**,
so the normalized multiple rises. Presenting those multiples alone made the
progress appear to reverse. They are also from different cohorts; the round-33
first pair is explicitly superseded and must not replace its confirmation.

For accepting each compiler edit, use its old/new variants measured in the same
balanced session. For long-term progress, report the fixed ruler and elapsed ms
alongside the fresh reference. The latest paired result is **4.93% less time**,
17.18× → 16.33× against one shared reference. The fixed-ruler result is 15.62×.

Source receipts: [round 32](../compression-round-32/primary/summary/manifest.json),
[round 33 first pair](../compression-round-33/primary/group0/manifest.json),
[round 33 confirmation](../compression-round-33/confirmed-primary/summary/manifest.json),
[round 34](../compression-round-34/primary/summary/manifest.json),
[round 35](../compression-round-35/primary/summary/manifest.json),
[round 36](primary/summary/manifest.json).
