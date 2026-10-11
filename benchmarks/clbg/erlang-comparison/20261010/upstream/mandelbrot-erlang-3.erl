% The Computer Language Benchmarks Game
% https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
%% Contributed by Johan Karlsson based on Fredrik Svahn's mandelbrot program
%% Added usage of port_command by Johan Karlsson

-module(mandelbrot).
-export([main/1]).
-define(LIM_SQR, 4.0).
-define(ITER, 50).
-define(SR, -1.5).
-define(SI, -1).

main([Arg]) ->
    N = list_to_integer(Arg),

    %% Start a printing process that prints using 
    %% port_command to decrease memory usage
    PrintProc = print_start(),
    print(PrintProc,["P4\n", Arg, " ", Arg, "\n"]),

    %% Spawn one process per row
    Row = fun(NextProc,Y)-> row(PrintProc, NextProc, N-1, 0, ?SI+Y*2/N, N, 0, [], 7) end,
    spawn_proc_chain(PrintProc,Row, N).

%% A function that spawns a chain of processes.
spawn_proc_chain(PrintProc,Row, N) ->
    Spawn = fun(PP,S,F,I,NN) -> spawn(fun() -> do_spawn_proc_chain(PP,S,F,I,NN) end) end,
    Spawn(PrintProc,Spawn,Row,first,N).

do_spawn_proc_chain(PrintProc,_,_,Max,Max) ->
    receive done -> ok end,
    %% Needed in order to let the print proccess finish printing.
    print_stop(PrintProc),
    halt(0);
do_spawn_proc_chain(PP,Spawn,Row,first,Max) ->
    NextProc = Spawn(PP,Spawn,Row,1,Max),
    %% I'm the first process in the chain. Inform my self that I can finish.
    self() ! done,
    %% Execute the row function
    Row(NextProc,0);
do_spawn_proc_chain(PP,Spawn,Row,N,Max) ->
    NextProc = Spawn(PP,Spawn,Row,N+1,Max),
    Row(NextProc,N).


%% Iterate over a row, collect bits, bytes and finally print the row
row(PrintProc, NextProc,X,X, _, _, Bits, Bytes, C) ->
    Char = case C of
               7 -> lists:reverse(Bytes);
               C -> lists:reverse([Bits bsl (C+1) | Bytes])
           end,
    %% Wait for the previous process to finish before printing
    receive _ -> ok end,
    print(PrintProc, Char),
    NextProc ! done;

row(PP, NP,M,X, Y2, N, Bits, Bytes, 0) ->
    row(PP, NP,M,X+1, Y2, N, 0, [Bits bsl 1 + m(?ITER, ?SR+(X+X)/N, Y2) | Bytes], 7);

row(PP, NP,M,X, Y2, N, Bits, Bytes, BitC) ->
    row(PP, NP,M,X+1, Y2, N, Bits bsl 1 + m(?ITER, ?SR+(X+X)/N, Y2), Bytes, BitC-1).


%Mandelbrot algorithm
m(Iter, CR,CI) -> m(Iter - 1, CR, CI, CR, CI).

m(Iter, R, I, CR, CI) ->
    case R*R+I*I > ?LIM_SQR of 
	false when Iter > 0 -> m(Iter-1, R*R-I*I+CR, 2*R*I+CI, CR, CI);
	false -> 1;
	true -> 0
    end.


%% Print stuff
print(PP,Char) ->
    PP ! Char.

print_start() ->
    spawn(fun() -> do_print_start() end).

print_stop(PrintProc) ->
    PrintProc ! {self(),stop},
    receive stopped -> ok end.

do_print_start() ->
    Fd = open_port({fd,0,1}, [out]),
    print_loop(Fd).

print_loop(Fd) ->
    receive {Pid,stop} ->
                Pid ! stopped;
            Char -> 
                port_command(Fd, Char), print_loop(Fd)
     end.
