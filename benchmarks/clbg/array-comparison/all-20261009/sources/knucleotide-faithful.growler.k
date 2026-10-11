\l /Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/array-comparison/all-20261009/stdlib/standard.k
/ Every window does a khash lookup and update. No query-only scans.
lines:0:0
start:1+*&{">THREE"~6#x}'lines
tail:start_lines
headers:&{">"~1#x}'tail
end:$[#headers;*headers;#tail]
dna:4!"ACGTacgt"?,/end#tail
inc:{[h;k]mapset[h;k;1+mapget[h;k]];h}
frame:{[job]
 width:job 0
 offset:job 1
 piece:job 2
 wanted:job 3
 total:_((#dna)-offset)%width
 first:_(total*piece)%8
 last:_(total*(piece+1))%8
 rows:last-first
 data:(width*rows)#(offset+width*first)_dna
 columns:+(0N;width)#data
 keys:{(4*x)+y}/[0;columns]
 h:mapnew 0
 inc/[h;keys]
 result:$[width<3;mapget[h]'!*/width#4;mapget[h;wanted]]
 mapclose h
 result}
count:{[width;wanted]
 jobs:,/{[width;wanted;offset]{[width;wanted;offset;piece](width;offset;piece;wanted)}[width;wanted;offset]'!8}[width;wanted]'!width
 +/peach[frame;jobs]}
decode:{[width;key]
 weights:|(*\1,(width-1)#4)
 d:4!_(key%weights)
 "ACGT"@d}
fixed:{s:$_0.5+1000*x;s:((0|4-#s)#"0"),s;(-3_s),".",-3#s}
frequency:{[width]
 counts:count[width;0]
 keys:&0<counts
 ordered:keys@<-counts@keys
 done:{[width;keys;counts;i]`0:decode[width;keys i]," ",fixed 100*(counts i)%+/counts}[width;keys;counts]'ordered
 `0:""
 ()}
done:frequency'1 2
query:{[s]
 codes:4!("ACGTacgt"?s)
 key:4/codes
 `0:($count[#s;key]),"\t",s
 ()}
done:query'("GGT";"GGTA";"GGTATT";"GGTATTTTAATT";"GGTATTTTAATTTATAGT")
