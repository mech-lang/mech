\l /Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/array-comparison/all-20261009/stdlib/standard.k
/ Every window does a khash lookup and update. No query-only scans.
lines:0:0
start:1+*&{">THREE"~6#x}'lines
tail:start_lines
headers:&{">"~1#x}'tail
end:$[#headers;*headers;#tail]
dna:4!"ACGTacgt"?,/end#tail
inc:{[h;k]mapset[h;k;1+mapget[h;k]];h}
update:{[h;width;frame]
 data:frame_dna
 rows:_(#data)%width
 columns:+(0N;width)#(width*rows)#data
 keys:{4*x+y}/[0;columns]
 inc/[h;keys]
 h}
count:{[width]
 `0:($width)," type ",$@width
 h:mapnew 0
 done:update[h;width]'!width
 h}
decode:{[width;key]"ACGT"@width#4\key}
fixed:{s:$_0.5+1000*x;s:((0|4-#s)#"0"),s;(-3_s),".",-3#s}
frequency:{[width]
 h:count width
 keys:mapkeys h
 keys:keys@<keys
 counts:mapget[h]'keys
 ordered:<-counts
 done:{[width;keys;counts;i]`0:decode[width;keys i]," ",fixed 100*(counts i)%+/counts}[width;keys;counts]'ordered
 `0:""
 mapclose h}
done:frequency'1 2
query:{[s]
 h:count #s
 key:4/4!"ACGTacgt"?s
 `0:($mapget[h;key]),"\t",s
 mapclose h}
done:query'("GGT";"GGTA";"GGTATT";"GGTATTTTAATT";"GGTATTTTAATTTATAGT")
