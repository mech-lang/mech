% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% All application work is here; standard MATLAB operations provide library support.
function fasta_optimized(n)
out = fopen('/dev/stdout','wb');
alu = uint8('GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA');
fprintf(out,'>ONE Homo sapiens alu\n');
for first = 0:60*16384:2*n-1
    width = min(60*16384,2*n-first);
    write_block(alu(mod(first+(0:width-1),numel(alu))+1),out);
end
seed = random_sequence('TWO IUB ambiguity codes',uint8('acgtBDHKMNRSVWY'),[0.27 0.12 0.12 0.27 repmat(0.02,1,11)],3*n,42,out);
random_sequence('THREE Homo sapiens frequency',uint8('acgt'),[0.3029549426680 0.1979883004921 0.1975473066391 0.3015094502008],5*n,seed,out);
fclose(out);
end
function seed = random_sequence(header,letters,probabilities,n,seed,out)
fprintf(out,'>%s\n',header);
edges = [0 cumsum(probabilities)];
for first = 1:60*16384:n
    width = min(60*16384,n-first+1);
    random = zeros(1,width);
    for pos = 1:width
        seed = mod(seed*3877+29573,139968);
        random(pos) = seed/139968;
    end
    write_block(letters(discretize(random,edges)),out);
end
end
function write_block(block,out)
whole = floor(numel(block)/60)*60;
output = [reshape(block(1:whole),60,[]); repmat(uint8(10),1,whole/60)];
fwrite(out,output,'uint8');
if whole < numel(block)
    fwrite(out,[block(whole+1:end) uint8(10)],'uint8');
end
end
