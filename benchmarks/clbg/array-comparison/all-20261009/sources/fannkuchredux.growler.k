/ Full enumeration and prefix reversal in canonical rotation order.
\l /Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/array-comparison/all-20261009/stdlib/standard.k
n:.`argv 2
facts:1,(*\1+!n)
p:!n
counts:n#0
index:0
checksum:0
maximum:0
flips:0
r:1
flip:{[q]size:1+*q;(|size#q),size_q}
flipcount:{flips::flips+1;flip x}
carry:{
 size:r+1
 p::(1_size#p),(*p),size_p
 exhausted:r=counts r
 counts[r]::$[exhausted;0;1+counts r]
 r::r+exhausted
 1-exhausted}
step:{
 flips::0
 done:{~0=*x}flipcount/p
 checksum::checksum+(1-2*2!index)*flips
 maximum::maximum|flips
 r::1
 done:{(0=x)&r<n}carry/0
 index::index+1
 r<n}
rank:0
stop:0
init:{[i]
 d:_(rank%(facts i))
 counts[i]::d
 rank::(facts i)!rank
 size:i+1
 prefix:size#p
 p::(d_prefix),(d#prefix),size_p}
work:{[span]
 p::!n
 counts::n#0
 index::span 0
 rank::span 0
 stop::span 1
 checksum::0
 maximum::0
 done:init'|1+!n-1
 done:{index<stop}step/1
 (checksum;maximum)}
cuts:_((!9)*(*|facts))%8
spans:+(-1_cuts;1_cuts)
results:peach[work;spans]
`0:$+/*'results
`0:"Pfannkuchen(",($n),") = ",$|/{x 1}'results
