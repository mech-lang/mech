% Generic source audit: inspect Erlang clause/block sequencing after preprocessing.
-module(loc_probe).
-export([main/1]).

main([Source]) ->
    {ok, Forms} = epp:parse_file(Source, [], []),
    Sequences = sequences(Forms),
    Counts = lists:foldl(fun(Body, Acc) ->
        Lines = lists:foldl(fun(Expr, M) ->
            Line = erl_anno:line(element(2, Expr)),
            maps:update_with(Line, fun(N) -> N + 1 end, 1, M)
        end, #{}, Body),
        maps:fold(fun(Line, N, M) ->
            case N > 1 of
                true -> maps:update_with(Line, fun(Extra) -> Extra + N - 1 end, N - 1, M);
                false -> M
            end
        end, Acc, Lines)
    end, #{}, Sequences),
    Output = [#{line => Line, extra => N} || {Line, N} <- lists:sort(maps:to_list(Counts))],
    io:put_chars(json:encode(Output)),
    halt().

sequences({clause, _, _, _, Body} = Term) -> [Body | children(Term)];
sequences({block, _, Body} = Term) -> [Body | children(Term)];
sequences(Term) when is_tuple(Term) -> children(Term);
sequences(Terms) when is_list(Terms) -> lists:append([sequences(T) || T <- Terms]);
sequences(_) -> [].

children(Term) -> sequences(tuple_to_list(Term)).
