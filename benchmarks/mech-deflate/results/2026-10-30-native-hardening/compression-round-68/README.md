# R68 rejected experiment

The descriptor-wide sharing candidate is rejected and reverted. It reduces allocation requests and observed memory, but increases matched elapsed and retired instructions. Its exact source/builds, successful semantic qualification, full allocation traces, adverse measurements, host assembly and diagnostic instruction windows remain retained. Current production returns to exact accepted R67 implementation source; two independent Core semantic regressions and source-contract guard repairs remain.

The candidate build source is f175488334a2c75e027de455f59d636bd2ffc502; its production change is e015670b7be26d3cc07d26640d1f538a0594f71b. Baseline build is c9199ed6687c0125f64e9bf2abd0b0057d5cbf0d at accepted checkpoint 5dff1496b523749e5f10f5085e2206be0c930a7e. Restoration is ae1113085. No rejected timing/counter result is labeled as production progress.

Run `python3 reconstruct.py /new/empty/output/path` from the public packet to verify every preserved archive version and reproduce eight numerical outputs without compilation, execution or external scratch inputs. Raw baseline/source-checker failures and the first incomplete guard repair are preserved; final checks and restored-production verification are separately labeled. Larger-input timing and full hosted CI are not claimed.
