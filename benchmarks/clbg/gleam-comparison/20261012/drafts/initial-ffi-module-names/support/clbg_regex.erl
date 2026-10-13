-module(clbg_regex).
-export([compile/1, search/3]).
compile(Source) ->
    {ok, Pattern} = re:compile(Source, []),
    Pattern.
search(Pattern, Data, Offset) ->
    case re:run(Data, Pattern, [{offset, Offset}, {capture, first, index}]) of
        {match, [{Start, Length}]} -> {ok, {Start, Length}};
        nomatch -> {error, nil};
        {error, Reason} -> erlang:error({regex_search, Reason})
    end.
