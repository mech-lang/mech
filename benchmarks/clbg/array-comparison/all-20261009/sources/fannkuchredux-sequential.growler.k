/ Full enumeration and prefix reversal in canonical rotation order.
n:.`argv 2
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
done:{x}step/1
`0:$checksum
`0:"Pfannkuchen(",($n),") = ",$maximum
