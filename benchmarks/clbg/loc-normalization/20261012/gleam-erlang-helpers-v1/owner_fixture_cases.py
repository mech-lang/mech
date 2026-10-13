"""Actual original-scanner fixture inputs; never synthetic parsed-form success."""

CASES = (
    ('named-header-and-value','f(X) -> X.',1,4),
    ('inline-first-callback','f() -> Pid = spawn_monitor(fun() -> Owner ! work() end), Pid.',0,7),
    ('expanded-first-callback','f() ->\n  Pid = spawn_monitor(\n    fun() ->\n      Owner ! work()\n    end),\n  Pid.',0,7),
    ('independent-callback-effect','f() -> Pid = spawn_monitor(fun() -> report(), Owner ! work() end), Pid.',0,8),
    ('two-real-callbacks','f() -> pair(fun() -> a() end, fun() -> b() end).',0,9),
    ('inline-receive-arm-body-close','f() -> receive {tag, V} -> V end.',0,7),
    ('expanded-receive-arm-body-close','f() ->\n  receive\n    {tag, V} ->\n      V\n  end.',0,7),
    ('nested-receive-two-outer-arms','f() -> receive {tag, V} -> receive ok -> V end; missing -> error end.',0,12),
    ('packed-case-pattern-and-data','f(X) -> case X of {a, V} -> {ok, V}; none -> error end.',1,9),
    ('independent-case-arm-effect','f(X) -> case X of {a, V} -> io:format("", []), {ok, V}; none -> error end.',1,10),
    ('runner-try-catch-two-effects','f() -> try benchmark:main() of _ -> ok catch C:R:S -> io:format("", []), halt(2) end.',0,10),
    ('literal-keywords-and-separators','f() -> {"receive fun -> end", \'case\', $;}. % case -> end',0,4),
    ('wrapped-data-retains-physical-row','f(X) -> {X,\n X}.',1,5),
    ('independent-chained-matches','f() -> X = Y = 1, X.',0,6),
    ('additional-anonymous-clause','f() -> fun(a) -> a; (b) -> b end.',0,8),
    ('unknown-authored-include','-include("foreign.hrl").\nf() -> ok.',0,None),
    ('actual-parser-error','f() -> receive {tag, V} -> V.',0,None),
    ('unknown-comprehension-owner','f() -> [X || X <- [1, 2]].',0,None),
    ('earlier-effect-before-callback','f() -> pair(io:format("", []), fun() -> ok end).',0,None),
    ('intervening-effect-between-callbacks','f() -> pair(fun() -> a() end, io:format("", []), fun() -> b() end).',0,None),
    ('written-selector-matches-remain-gated','f() -> case X = Y = 1 of _ -> ok end.',0,None),
)

EXPECTED_IDS = [value[0] for value in CASES]
