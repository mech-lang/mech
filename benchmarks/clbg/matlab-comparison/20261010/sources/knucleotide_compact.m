% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% All application work is here; standard MATLAB operations provide library support.
function knucleotide_compact()
lines = readlines('/dev/fd/3','EmptyLineRule','skip');
first = find(startsWith(lines,'>THREE'),1);
last = find(startsWith(lines(first+1:end),'>'),1);
if isempty(last)
    last = numel(lines)-first;
else
    last = last-1;
end
sequence = uint8(upper(char(join(lines(first+1:first+last),''))));
map = zeros(1,256);
map(uint8('ACGT')+1) = 0:3;
codes = map(sequence+1);
alphabet = 'ACGT';
widths = [1 2 3 4 6 12 18];
queries = ["" "" "GGT" "GGTA" "GGTATT" "GGTATTTTAATT" "GGTATTTTAATTTATAGT"];
for j = 1:numel(widths)
    width = widths(j);
    words = zeros(1,numel(codes)-width+1);
    for offset = 1:width
        words = 4*words+codes(offset:end-width+offset);
    end
    counts = dictionary(words,0);
    for frame = 1:width
        counts = update_frame(counts,words,frame,width);
    end
    assert(sum(double(values(counts))) == numel(words));
    if width <= 2
        word = keys(counts);
        frequency = values(counts);
        sorted = sortrows([-frequency word]);
        for row = 1:size(sorted,1)
            key = sorted(row,2);
            text = repmat('A',1,width);
            for pos = width:-1:1
                text(pos) = alphabet(mod(key,4)+1);
                key = floor(key/4);
            end
            fprintf('%s %.3f\n',text,-100*sorted(row,1)/numel(words));
        end
        fprintf('\n');
    else
        query = map(uint8(char(queries(j)))+1);
        key = sum(query.*4.^(width-1:-1:0));
        fprintf('%d\t%s\n',lookup(counts,key,'FallbackValue',0),queries(j));
    end
end
end
function counts = update_frame(counts,words,frame,width)
for pos = frame:width:numel(words)
    key = words(pos);
    counts(key) = counts(key)+1;
end
end
