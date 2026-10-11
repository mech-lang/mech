#!/usr/bin/env amber
/Amber entry Sep 2026
x:1:"/dev/stdin";h:(&x=">")2;s:1+h+(x@h+!200)?"\n";N:#x;K:262144;P:"ggtattttaatttatagt"
f:{[S;o]b:x@o+!K&N-o;b:b@&b>"\n";d:(S 0),b;L:#S 0;p:(-1_4*c)+1_c:"acgt"?d
 g:&p=10;g:g@&g<$[N>o+K;0|(#d)-17;#d]
 (-17#d;(S 1)+0^(#'=(0|L-1)_p)@!16;(S 2)+#'{[d;x;y]x@&P[y]=d@x+y}[d]\[g;2+!16];(S 3)+#b;"acgt"?*|d)}
Z:f/[(();16#0;16#0;0;0);s+K*!-_-(N-s)%K];c2:Z 1;n:Z 3
fm:{[k;c;n]{x," ",(-3_r),".",-3#r:$_0.5+1e5*y}'[k;c%n]@>c}
`0:(fm[,'"ACGT";(+/'4 4#c2)+(!4)=Z 4;n]),,""
`0:(fm["ACGT"@+4 4\!16;c2;n-1]),,""
S:" "\"GGT GGTA GGTATT GGTATTTTAATT GGTATTTTAATTTATAGT"
`0:"\t"/'+($(Z 2)@-3+#'S;S)
