% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% All application work is here; standard MATLAB operations provide library support.
function knucleotide_optimized()
fid = fopen('/dev/fd/3','rb');
line = fgetl(fid);
while ischar(line) && ~startsWith(line,'>THREE')
    line = fgetl(fid);
end
chunks = cell(1,1024);
used = 0;
line = fgetl(fid);
while ischar(line) && ~startsWith(line,'>')
    used = used+1;
    if used > numel(chunks)
        chunks{2*numel(chunks)} = [];
    end
    chunks{used} = uint8(upper(line));
    line = fgetl(fid);
end
fclose(fid);
sequence = [chunks{1:used}];
clear chunks;
map = zeros(1,256,'uint64');
map(uint8('ACGT')+1) = uint64(0:3);
codes = map(sequence+1);
clear sequence;
widths = [1 2 3 4 6 12 18];
queries = ["" "" "GGT" "GGTA" "GGTATT" "GGTATTTTAATT" "GGTATTTTAATTTATAGT"];
alphabet = 'ACGT';
for j = 1:numel(widths)
    width = widths(j);
    words = zeros(1,numel(codes)-width+1,'uint64');
    for offset = 1:width
        words = bitor(bitshift(words,2),codes(offset:end-width+offset));
    end
    counts = configureDictionary('uint64','uint32');
    for frame = 1:width
        counts = update_frame(counts,words,frame,width);
    end
    assert(sum(double(values(counts))) == numel(words));
    if width <= 2
        word = keys(counts);
        frequency = double(values(counts));
        sorted = sortrows([-frequency double(word)]);
        for row = 1:size(sorted,1)
            key = uint64(sorted(row,2));
            text = repmat('A',1,width);
            for pos = width:-1:1
                text(pos) = alphabet(double(bitand(key,3))+1);
                key = bitshift(key,-2);
            end
            fprintf('%s %.3f\n',text,-100*sorted(row,1)/numel(words));
        end
        fprintf('\n');
    else
        query = map(uint8(char(queries(j)))+1);
        key = uint64(0);
        for code = query
            key = bitor(bitshift(key,2),code);
        end
        fprintf('%d\t%s\n',lookup(counts,key,'FallbackValue',uint32(0)),queries(j));
    end
    clear counts words;
end
end
function counts = update_frame(counts,words,frame,width)
for pos = frame:width:numel(words)
    key = words(pos);
    counts(key) = lookup(counts,key,'FallbackValue',uint32(0))+uint32(1);
end
end
