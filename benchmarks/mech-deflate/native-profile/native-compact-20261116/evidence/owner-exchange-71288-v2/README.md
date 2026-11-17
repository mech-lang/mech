# Complete v2 timing of the 71,288-byte owner-exchange encoder

The plan freezes the 71,768-byte control, 71,288-byte candidate, peer
executables, inputs, worker orders and policy before acquisition. Both Apple
M1 orders complete all 81 named cases and 982,656 observed calls apiece.
[Plan and hashes](backend-work-hardened/source-owner-exchange-71288-v2/plan.json)
· [Complete report](backend-work-hardened/source-owner-exchange-71288-v2/full-replay.json)

| Order | Roc wins control → candidate | libdeflate wins control → candidate |
| --- | ---: | ---: |
| Forward | 81 → 81 | 61 → 61 |
| Reverse | 81 → 81 | 60 → 60 |

The reverse order loses the control's 1 KiB random-data win. The paired
estimate moves from 0.99585× libdeflate for the control to 1.00195× for
the candidate. The candidate gains the 320-byte overlap win in that order.
The point estimates are near parity. Both workers also miss the 61-win
absolute floor in reverse. Policy v2 rejects the candidate on the
individual loss. The 71,768-byte encoder remains the reference.
[Forward summaries](source-owner-exchange-71288-v2-full-forward/summary.json)
· [Reverse summaries](source-owner-exchange-71288-v2-full-reverse/summary.json)

The candidate's empty-input time rises 1.30% in the combined paired
estimate. Policy v2 records this interval as a diagnostic. The empty-input
peer wins hold in both orders. The historical v1 outcomes remain in the
[earlier focused study](../owner-exchange-71288/README.md).

Run the following command within this directory to recheck fixture hashes,
binary identity records and all original observation blocks under policy v2.
The recorded Roc worker is hashed by the timer before launch. Its archived
metadata digest and invocation are retained. The legacy Roc metadata file
itself was unavailable for rehashing.

~~~sh
python3 -B replay.py
~~~
