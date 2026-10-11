/ Preserve FASTA headers, reverse each complete sequence, and wrap at 60 columns.
lines:0:"/dev/stdin"
starts:&">"=*'lines
boundaries:starts,#lines
alphabet:"ACBDGHKMNSRUTWVYacbdghkmnsrutwvy"
complement:"TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"
record:{[ab]
 a:*ab
 b:*|ab
 `0:lines a
 sequence:,/lines@(a+1)+!(b-a)-1
 result:|complement@alphabet?sequence
 full:60*_(#result)%60
 `1:,/{x,"\n"}'0N 60#full#result
 $[full<#result;`0:full_result;()]}
done:record'(-1_boundaries),'1_boundaries
