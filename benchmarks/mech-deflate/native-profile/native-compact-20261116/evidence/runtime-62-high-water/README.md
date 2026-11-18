# Runtime high-water mark

The archived register-rematerialization worker records 81/81 Roc and
62/81 libdeflate wins in both complete orders. Its encoder came from
the earlier LLVM-assisted pipeline. The maintained LLVM-free worker
records 81/81 and 61/81. The two campaigns use identical input inventories
and peer workers. All 81 output hashes and capacities agree in each order.

Natural 8 KiB is the only individual peer win missing from the maintained
worker. Natural 4 KiB is the next closest loss in the latest scorer work.

| Order | Case | Archived 62-win worker vs C | Maintained worker vs C |
| --- | --- | ---: | ---: |
| Forward | prefix-natural-4096 | 1.0520× | 1.1188× |
| Forward | prefix-natural-8192 | 0.9828× | 1.0352× |
| Reverse | prefix-natural-4096 | 1.0635× | 1.1324× |
| Reverse | prefix-natural-8192 | 0.9881× | 1.0410× |

The v3 release policy requires 81/81 Roc and 62/81 libdeflate in each
complete order, including natural 8 KiB. Each order must also keep the
peer wins of its contemporaneous control. The next target is 63/81.
[Policy](../../qualification-policy-v3.json) · [Bound score comparison](summary.json)
