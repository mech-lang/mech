%% Counted reusable I/O/formatting glue. No domain algorithm is implemented here.
-module(clbg_io).
-export([argument/0, read_all/0, write/1, text/1, decimal/2, join/1, bytes/1]).
argument() ->
    case init:get_plain_arguments() of
        [Value | _] -> list_to_integer(Value);
        [] -> 0
    end.
read_all() ->
    ok = io:setopts(standard_io, [binary, {encoding, latin1}]),
    read_all([]).
read_all(Parts) ->
    case file:read(standard_io, 65536) of
        {ok, Bytes} -> read_all([Bytes | Parts]);
        eof -> iolist_to_binary(lists:reverse(Parts));
        {error, Reason} -> erlang:error({stdin_read, Reason})
    end.
write(Bytes) ->
    ok = file:write(standard_io, Bytes),
    nil.
text(Bytes) -> write(Bytes).
decimal(Value, Precision) ->
    iolist_to_binary(io_lib:format("~.*f", [Precision, Value])).
join(Parts) -> iolist_to_binary(Parts).
bytes(Values) -> list_to_binary(Values).
