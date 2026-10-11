% The Computer Language Benchmarks Game
% https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
%   contributed by Isaac Gouy (Erlang novice)

-module(spectralnorm).
-export([main/1]).

main([Arg]) ->
    N = list_to_integer(Arg),
    {U,V} = powerMethod(N,10, array(1.0,N,[]), array(0.0,N,[]) ),
    io:format("~.9f\n",[ loop(N,U,V,0.0,0.0) ]),
    erlang:halt(0).

% eigenvalue of V
loop(0,_,_,VBV,VV) -> math:sqrt(VBV/VV);
loop(I,U,V,VBV,VV) -> 
   VI = element(I,V),
   loop(I-1,U,V, VBV + element(I,U)*VI, VV + VI*VI).

% 2I steps of the power method
powerMethod(_,0,A,B) -> {A,B};
powerMethod(N,I,A,B) -> 
   V = atav(N,A,B), 
   U = atav(N,V,A),
   powerMethod(N,I-1,U,V).

% return element i,j of infinite matrix A
a(II,JJ) ->
   I = II-1.0, J = JJ-1.0,
   1.0/((I+J)*(I+J+1.0)/2.0 +I+1.0).

% multiply vector v by matrix A
av(_,0,_,AV) -> AV;
av(N,I,V,AV) -> 
   av(N,I-1,V, setelement(I,AV, avloop(N,I,V,0.0) )).

avloop(0,_,_,X) -> X;
avloop(J,I,V,X) -> 
   avloop(J-1,I,V, X + a(I,J)*element(J,V) ).

% multiply vector v by matrix A transposed
atv(_,0,_,ATV) -> ATV;
atv(N,I,V,ATV) -> 
   atv(N,I-1,V, setelement(I,ATV, atvloop(N,I,V,0.0) )).

atvloop(0,_,_,X) -> X;
atvloop(J,I,V,X) -> atvloop(J-1,I,V, X + a(J,I)*element(J,V) ).

% multiply vector v by matrix A and then by matrix A transposed
atav(N,V,ATAV) ->
   atv(N,N, av(N,N,V,array(0.0,N,[])) ,ATAV).

% initialize a list and convert it to a tuple
array(_,0,L) -> list_to_tuple(L);
array(X,N,L) -> array(X,N-1,[X|L]).
