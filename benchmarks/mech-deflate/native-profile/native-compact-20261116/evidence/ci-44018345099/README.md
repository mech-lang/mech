# Integration follow-up for GitHub Actions run 44018345099

The hosted browser standard job reached the rich-document resize test.
In Chrome 157 the archived reproduction holds native fullscreen exit for
two seconds, and the original assertion reads a 1,680-pixel fullscreen pane.
The updated assertion waits for native exit and docked mode. Both native
and CSS fallback then start at 620 pixels and pass the original drag checks.
The reproduction uses the shipped controller and stylesheet from the hosted
diagnostic. It exercises the affected assertions in isolation.

The static architecture job failed on an archived matrix worker. The existing
checker accepts the 11 added exact-file entries, and its mutation fixtures
pass. The complete head audit still fails at the frozen historical runtime.
The inventory also records two maintained types with manual thread-safety
implementations and many exact-file omissions already present in the base.
The census continued after each failure to collect all locations. Its final
line is from the diagnostic copy, while the ordinary checker result is in
the before and after logs.

- [Run](https://github.com/mech-lang/mech/actions/runs/44018345099)
- [Browser standard job](https://github.com/mech-lang/mech/actions/runs/44018345099/job/133822073347)
- [Static architecture job](https://github.com/mech-lang/mech/actions/runs/44018345099/job/133822073430)
