%% Generic fresh-process dispatch only; benchmark:main/0 is authored Gleam.
-module(clbg_runner).
-export([main/0]).
main() ->
    try benchmark:main() of
        _ -> halt(0)
    catch Class:Reason:Stack ->
        io:format(standard_error, "~p:~p~n~p~n", [Class, Reason, Stack]),
        halt(2)
    end.
