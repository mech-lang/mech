% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% Compact and optimized candidates; full required work, no native benchmark executor.
function revcomp_optimized()
out = fopen('/dev/stdout','wb');
fid = fopen('/dev/fd/3','rb');
raw = fread(fid,Inf,'*uint8')';
fclose(fid);
headers = find(raw == uint8('>'));
map = zeros(1,256,'uint8');
map(uint8('ABCDEFGHIJKLMNOPQRSTUVWXYZ')+1) = uint8('TVGHEFCDIJMLKNOPQYSAABWXRZ');
map(uint8('abcdefghijklmnopqrstuvwxyz')+1) = map(uint8('ABCDEFGHIJKLMNOPQRSTUVWXYZ')+1);
for record = 1:numel(headers)
    first = headers(record);
    last = numel(raw);
    if record < numel(headers)
        last = headers(record+1)-1;
    end
    newline = first+find(raw(first:last) == 10,1)-1;
    fwrite(out,raw(first:newline),'uint8');
    sequence = raw(newline+1:last);
    sequence = sequence(sequence ~= 10 & sequence ~= 13);
    sequence = map(sequence(end:-1:1)+1);
    for first_base = 1:60*16384:numel(sequence)
        block = sequence(first_base:min(first_base+60*16384-1,end));
        whole = floor(numel(block)/60)*60;
        output = [reshape(block(1:whole),60,[]); repmat(uint8(10),1,whole/60)];
        fwrite(out,output,'uint8');
        if whole < numel(block)
            fwrite(out,[block(whole+1:end) uint8(10)],'uint8');
        end
    end
end
fclose(out);
end
