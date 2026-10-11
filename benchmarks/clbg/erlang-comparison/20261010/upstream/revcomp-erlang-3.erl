% The Computer Language Benchmarks Game
% https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
%%
%% contributed by Vlad Balin
%% optimizations by Fredrik Svahn

-module(revcomp).
-compile( [ inline, { inline_size, 100 } ] ).

-export([main/1]).

main([_Args]) ->
    register(print_server, self()),
    print_server ! flush_queue_empty,
    io:setopts( [ binary ] ),
    loop([]),
    halt().

loop( Buffer ) ->
    case io:get_line('') of
        eof -> 	    
	    % block until previous output process is done
	    receive flush_queue_empty -> ok end,
	    flush( Buffer, << >> );
        << ">", _/bytes >> = Head ->
	    % block until previous output process is done
	    receive flush_queue_empty -> ok end,
	    % spawn output process and continue to read in main thread
            spawn(fun()-> flush( Buffer, Head ) end),
	    loop( [] );
        Line -> loop( [ rev_comp_line( Line, <<>> ) | Buffer] )
    end.

%% flush( Buffer, Suffix ) -> atom().
%% Buffer = Suffix = iolist().
%% Format and write Buffer with sequence followed by Suffix text
flush( Buffer, Suffix ) ->
    io:put_chars( format( iolist_to_binary(Buffer), Suffix ) ),
    print_server ! flush_queue_empty.

%% format( Buffer, Suffix ) -> iolist().
%% Buffer = bytes(), Suffix = iolist().
%% Split Buffer into 60-char lines, append Suffix to the end of buffer.
format( << Line:60/bytes, Rest/bytes >>, Suffix ) -> [ Line, 10 | format( Rest, Suffix ) ];
format( << >>, Suffix ) -> Suffix;
format( Line, Suffix ) -> [ Line, 10, Suffix ].

%% rev_comp_line( Line, Buffer ) -> Buffer.
%% Line = binary().
%% Buffer = binary().
rev_comp_line( << _:8 >>, Buffer ) -> Buffer;
rev_comp_line( << H, Rest/bytes >>, Buffer ) -> 
    C = rev_comp( H ),
    rev_comp_line( Rest, << C:8, Buffer/binary >> ).

rev_comp( $A ) -> $T;
rev_comp( $C ) -> $G;
rev_comp( $G ) -> $C;
rev_comp( $T ) -> $A;
rev_comp( $U ) -> $A;
rev_comp( $M ) -> $K;
rev_comp( $R ) -> $Y;
rev_comp( $Y ) -> $R;
rev_comp( $K ) -> $M;
rev_comp( $V ) -> $B;
rev_comp( $H ) -> $D;
rev_comp( $D ) -> $H;
rev_comp( $B ) -> $V;
rev_comp( $a ) -> $T;
rev_comp( $c ) -> $G;
rev_comp( $g ) -> $C;
rev_comp( $t ) -> $A;
rev_comp( $u ) -> $A;
rev_comp( $m ) -> $K;
rev_comp( $r ) -> $Y;
rev_comp( $y ) -> $R;
rev_comp( $k ) -> $M;
rev_comp( $v ) -> $B;
rev_comp( $h ) -> $D;
rev_comp( $d ) -> $H;
rev_comp( $b ) -> $V;
rev_comp( $N ) -> $N;
rev_comp( $S ) -> $S;
rev_comp( $W ) -> $W;
rev_comp( $n ) -> $N;
rev_comp( $s ) -> $S;
rev_comp( $w ) -> $W;
rev_comp( _ ) -> $?.
