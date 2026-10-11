% The Computer Language Benchmarks Game
% https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
%%
%% Contributed by : Alkis Gotovos 10 Oct 2010
%% Some misc minor optimizations by Johan Karlsson

-module(pidigits).

-compile([inline, native, {hipe, [o3]}]).

-compile({inline_size, 100}).

-export([main/1]).

main([N]) -> main(list_to_integer(N));
main(N) ->
    Pid = spawn(fun () -> io_worker() end),
    register(io_worker, Pid),
    stream({1, 0, 1}, 1, 0, N).

prod({Z11, Z12, Z22}, N) ->
    {10 * Z11, 10 * (Z12 - N * Z22), Z22}.

stream(Z, K, P, N) ->
    {Q, R, T} = Z,
    Q3R = 3 * Q + R,
    Y = Q3R div T,
    Safe = Y == (Q3R + Q) div T,
    case Safe of
        true ->
            PNext = P + 1,
	        io_worker ! {Y, PNext, N},
	        stream(prod(Z, Y), K, PNext, N);
        false ->
	        K2 = 2 * K,
            K21 = K2 + 1,
            New = {Q * K, Q * (2 * K2 + 2) + R * K21, T * K21},
	        stream(New, K + 1, P, N)
    end.


io_worker() ->
    Fd = open_port({fd, 0, 1}, [out]),
    do_io_work(Fd).

do_io_work(Fd) ->
    receive
        {Y, N, N} ->
            Spaces = (10 - N rem 10) rem 10,
            port_command(Fd, io_lib:format("~w~.*c\t:~w~n", [Y, Spaces, $\s, N])),
	        erlang:halt(0);
        {Y, P, _N} when P rem 10 == 0 ->
	        port_command(Fd, io_lib:format("~w\t:~w~n", [Y, P])),
	        do_io_work(Fd);
        {Y, _P, _N} ->
	        port_command(Fd, io_lib:format("~w", [Y])),
	        do_io_work(Fd)
    end.
