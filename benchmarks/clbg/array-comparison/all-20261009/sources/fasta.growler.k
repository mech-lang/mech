/ CLBG LCG, inverse cumulative probabilities, and 60-column FASTA output.
n:.`argv 2
seed:42
alu:"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
wrap:{[s]
 full:60*_(#s)%60
 `1:,/{x,"\n"}'0N 60#full#s
 $[full<#s;`0:full_s;()]}
rand:{139968!29573+3877*x}
random:{[alphabet;probabilities;size]
 r:1_size rand\seed
 seed::*|r
 cumulative:+\probabilities
 indices:+/{[r;t]~t>r}[r%139968]'cumulative
 wrap alphabet@indices}
`0:">ONE Homo sapiens alu"
done:wrap (2*n)#alu
`0:">TWO IUB ambiguity codes"
done:random["acgtBDHKMNRSVWY";0.27 0.12 0.12 0.27,11#0.02;3*n]
`0:">THREE Homo sapiens frequency"
done:random["acgt";0.3029549426680 0.1979883004921 0.1975473066391 0.3015094502008;5*n]
