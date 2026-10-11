% The Computer Language Benchmarks Game
% https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
% Contributed by: Hynek Vychodil 2009
% Fixed by Alexander Fyodorov


% Inspired by regex-dna Erlang HiPE #5 program
%    by Sergei Matusevich 2007 and Thanassis Avgerinos 2009

% Main changes:
%   1/ Very fast Port line input instead stdio (~5x)
%   2/ Faster IUB code alternatives explicit expansion
%      using binary instead lists (~5x)
%   3/ Precompile regexps in data loading phase
%   4/ Simpler dispatch and result join code

% Note: re module is damn slow. Boyer-Moore like binary matcher
% written in Erlang should be magnitude faster (HiPE of course).

-module(regexredux).

-compile([native, {hipe, [o3]}]).

-export([main/1]).

main(_) -> main().

main() -> do(), halt().

do() ->
    S = self(),
    Worker = spawn_link(fun () -> work(S) end),
    Worker ! {data, read()},
    receive finish -> ok end.

work(Master) ->
    S = self(),
    Patterns = [{Pat, re:compile(Pat, [caseless])}
		|| Pat <- patterns()],
    {RawSize, [B3, B2, B1 | _]} = receive
				    {data, Data} -> Data
				  end,
    L = [size(X) || X <- [B1, B2, B3]],
    Size = lists:sum(L),
    PIDS = [{spawn_link(matcher(S, B2, B3, MR)),
	     printer(Pat)}
	    || {Pat, {ok, MR}} <- Patterns],
    Final = apply_final_patterns([B2,B3], final_patterns()),
    FinalSize = size(B1) + size(Final),
    results(PIDS),
    io:format("~n~b~n~b~n~b~n",
	      [RawSize, Size, FinalSize]),
    Master ! finish.

apply_final_patterns(S, [{Pat, Rep}|Rest]) ->
  apply_final_patterns(re:replace(S, Pat, Rep,[{return,binary},caseless,global,never_utf]), Rest);
apply_final_patterns(S, []) ->
  S.

matcher(S, B2, B3, MR) ->
    fun () ->
	    S !
	      {self(), countMatches(B2, MR) + countMatches(B3, MR)}
    end.

printer(Pat) ->
    fun (Num) -> io:format("~s ~b~n", [Pat, Num]) end.

countMatches(Data, RE) ->
    case re:run(Data, RE, [global]) of
      {match, M} -> length(M);
      nomatch -> 0
    end.

results([{PID, Fin} | R]) ->
    receive {PID, Ret} -> Fin(Ret), results(R) end;
results([]) -> ok.

patterns() ->
    ["agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]",
     "a[act]ggtaaa|tttacc[agt]t",
     "ag[act]gtaaa|tttac[agt]ct",
     "agg[act]taaa|ttta[agt]cct",
     "aggg[acg]aaa|ttt[cgt]ccct",
     "agggt[cgt]aa|tt[acg]accct",
     "agggta[cgt]a|t[acg]taccct",
     "agggtaa[cgt]|[acg]ttaccct"].

final_patterns() ->
  [
    {"tHa[Nt]", "<4>"},
    {"aND|caN|Ha[DS]|WaS", "<3>"},
    {"a[NSt]|BY", "<2>"},
    {"<[^>]*>", "|"},
    {"\\|[^|][^|]*\\|", "-"}
  ].

read() ->
    Port = open_port({fd, 0, 1}, [in, binary, {line, 256}]),
    read(Port, 0, [], []).

read(Port, Size, Seg, R) ->
    receive
      {Port, {data, {eol, <<$>:8, _/binary>> = Line}}} ->
	  read(Port, Size + size(Line) + 1, [],
	       [iolist_to_binary(lists:reverse(Seg, [])) | R]);
      {Port, {data, {eol, Line}}} ->
	  read(Port, Size + size(Line) + 1, [Line | Seg], R);
      {'EXIT', Port, normal} ->
	  {Size, [iolist_to_binary(lists:reverse(Seg, [])) | R]};
      Other ->
	  io:format(">>>>>>> Wrong! ~p~n", [Other]),
	  exit(bad_data)
    end.
