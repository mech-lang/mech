% The Computer Language Benchmarks Game
% https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
%
% contributed by Isaac Gouy (Erlang novice)
% reset

-module(binarytrees). 
-export([main/1]). 

-define(Min,4).

main([Arg]) ->
   N = list_to_integer(Arg),
   Max = lists:max([?Min+2,N]),

   Stretch = Max + 1,
   io:fwrite("stretch tree of depth ~w\t check: ~w~n", 
      [ Stretch, itemCheck(bottomUp(Stretch)) ]),

   LongLivedTree = bottomUp(Max),
   depthLoop(?Min,Max),

   io:fwrite("long lived tree of depth ~w\t check: ~w~n", 
      [ Max, itemCheck(LongLivedTree) ]),

   halt(0).


depthLoop(D,M) when D > M -> ok;
depthLoop(D,M) -> 
   N = 1 bsl (M-D + ?Min),
   io:fwrite("~w\t trees of depth ~w\t check: ~w~n", 
      [ N, D, sumLoop(N,D,0) ]),
   depthLoop (D+2,M).

sumLoop(0,_,Sum) -> Sum;
sumLoop(N,D,Sum) -> 
   sumLoop(N-1,D, Sum + itemCheck(bottomUp(D))).

bottomUp(0) -> {nil, nil};
bottomUp(D) -> {bottomUp(D-1), bottomUp(D-1)}.

itemCheck(nil) -> 0;
itemCheck({Left,Right}) -> 
   1 + itemCheck(Left) + itemCheck(Right).
