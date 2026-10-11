% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% Compact and optimized candidates; full required work, no native benchmark executor.
function revcomp_compact()
out = fopen('/dev/stdout','wb');
lines = readlines('/dev/fd/3','EmptyLineRule','skip');
headers = find(startsWith(lines,'>'));
map = uint8(zeros(1,256));
map(uint8('ABCDEFGHIJKLMNOPQRSTUVWXYZ')+1) = uint8('TVGHEFCDIJMLKNOPQYSAABWXRZ');
map(uint8('abcdefghijklmnopqrstuvwxyz')+1) = map(uint8('ABCDEFGHIJKLMNOPQRSTUVWXYZ')+1);
for record = 1:numel(headers)
    first = headers(record);
    last = numel(lines);
    if record < numel(headers)
        last = headers(record+1)-1;
    end
    fprintf(out,'%s\n',lines(first));
    dna = uint8(char(join(lines(first+1:last),'')));
    write_sequence(map(dna(end:-1:1)+1),out);
end
fclose(out);
end
function write_sequence(dna,out)
for pos = 1:60:numel(dna)
    fwrite(out,[dna(pos:min(pos+59,end)) uint8(10)],'uint8');
end
end
