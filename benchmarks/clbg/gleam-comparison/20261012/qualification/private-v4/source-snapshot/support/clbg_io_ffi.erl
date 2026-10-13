%% Counted reusable I/O/formatting glue. No domain algorithm is implemented here.
-module(clbg_io_ffi).
-export([argument/0, read_all/0, write/1, text/1, decimal/2, join/1, bytes/1]).
argument() ->
    case init:get_plain_arguments() of
        [Value | _] -> list_to_integer(Value);
        [] -> 0
    end.
read_all() ->
    Port = open_port({fd, 0, 1}, [in, binary, eof]),
    read_all(Port, []).
read_all(Port, Parts) ->
    receive
        {Port, {data, Bytes}} -> read_all(Port, [Bytes | Parts]);
        {Port, eof} ->
            true = port_close(Port),
            iolist_to_binary(lists:reverse(Parts))
    end.
write(Bytes) ->
    ok = file:write(standard_io, Bytes),
    nil.
text(Bytes) -> write(Bytes).
decimal(Value, Precision) ->
    iolist_to_binary(io_lib:format("~.*f", [Precision, Value])).
join(Parts) -> iolist_to_binary(Parts).
bytes(Values) -> list_to_binary(Values).
