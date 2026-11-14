# Fold the branch after equal join arguments

The first equal-argument trial left a constant false branch in the matcher. This variant runs the existing constant branch fold after the equal-argument proof. Reachability and phi cleanup retire the last-match load block. The actual literal read, cache capacity check, header store, and cursor advance still run in their original order. Exactly one branch folds in the benchmark. One of 351 bodies changes. The matcher drops from 10,284 to 10,188 bytes and keeps a 576-byte frame. Static stack reads fall from 358 to 345 and writes rise from 105 to 107.

The independent screen compares this variant against the selected-read control from `f3a628b73cb01a7a7d8ee034714ec02b41674a5f`. It measures **0.993022× control** across nine cases and 6,912 calls. Both 1.05 timing gates pass on all nine cases. The same six cases beat Roc, and periodic 1 MiB and repeated 128 KiB add libdeflate wins. Five cases beat both peers in this screen. The full qualification uses the published reverse cursor image as its control.

| Input | Control/Roc | Trial/Roc | Control/libdeflate | Trial/libdeflate | Trial/control |
| --- | ---: | ---: | ---: | ---: | ---: |
| Eight literals | 0.171584× | 0.171628× | 2.204261× | 2.204829× | 1.000257× |
| Natural 8 KiB | 1.048128× | 1.045628× | 1.301510× | 1.298405× | 0.997614× |
| Natural 1 MiB | 1.191220× | 1.192664× | 0.960702× | 0.961866× | 1.001212× |
| Random 1 MiB | 1.411259× | 1.415682× | 1.994460× | 2.000711× | 1.003134× |
| Periodic 1 MiB | 0.591999× | 0.584324× | 1.002942× | 0.989938× | 0.987035× |
| Repeated 32,767 bytes | 0.509826× | 0.505498× | 0.996089× | 0.987634× | 0.991511× |
| Repeated 128 KiB | 0.600795× | 0.592607× | 1.006182× | 0.992469× | 0.986371× |
| Repeated 256 KiB | 0.607717× | 0.599106× | 0.991182× | 0.977137× | 0.985830× |
| Repeated 1 MiB | 0.605420× | 0.596011× | 0.949350× | 0.934597× | 0.984460× |

The independent counter cohort puts periodic 1 MiB at 0.983566× the control's instructions and 0.982931× its cycles. Repeated 1 MiB measures 0.983939× instructions and 0.983653× cycles. Random 1 MiB retires 0.21% more instructions. Eight-literal cycles increase 2.27% in that cohort. All six payload metrics match. Both original linked workers pass 162 complete checks with exact bytes, independent inflate, held aliases, capacity, and cleanup. The archive contains the linked branch audit, nine profiles, raw packets, and the earlier 700 focused controls for the same argument proof.

[Nine-case pilot, compiler diff, and timing replay](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/mech-f3a628b73c-equal-join-fold-pilot.tar.gz). Archive SHA-256 `e8678f408e63f29c6212b5f7462b35c7c2525549bbc8da1c65943919496a8c50`.
