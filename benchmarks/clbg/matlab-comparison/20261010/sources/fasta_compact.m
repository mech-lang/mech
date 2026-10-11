% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% All application work is here; standard MATLAB operations provide library support.
function fasta_compact(n)
out = fopen('/dev/stdout','wb');
alu = uint8('GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA');
fprintf(out,'>ONE Homo sapiens alu\n');
for first = 0:60:2*n-1
    width = min(60,2*n-first);
    fwrite(out,[alu(mod(first+(0:width-1),numel(alu))+1) uint8(10)],'uint8');
end
seed = random_sequence('TWO IUB ambiguity codes',uint8('acgtBDHKMNRSVWY'),[0.27 0.12 0.12 0.27 repmat(0.02,1,11)],3*n,42,out);
random_sequence('THREE Homo sapiens frequency',uint8('acgt'),[0.3029549426680 0.1979883004921 0.1975473066391 0.3015094502008],5*n,seed,out);
fclose(out);
end
function seed = random_sequence(header,letters,probabilities,n,seed,out)
fprintf(out,'>%s\n',header);
cumulative = cumsum(probabilities);
for first = 1:60:n
    width = min(60,n-first+1);
    line = repmat(uint8(10),1,width+1);
    for pos = 1:width
        seed = mod(seed*3877+29573,139968);
        random = seed/139968;
        selected = 1;
        while random >= cumulative(selected)
            selected = selected+1;
        end
        line(pos) = letters(selected);
    end
    fwrite(out,line,'uint8');
end
end
