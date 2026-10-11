function dictionary_probe(n,cardinality)
% Untimed generic dictionary API diagnostic; never a chart benchmark sample.
for numeric = 1:2
    words = mod(0:n-1,cardinality)*65537;
    if numeric == 2
        words = uint64(words);
        zero = uint32(0);
        one = uint32(1);
    else
        zero = 0;
        one = 1;
    end
    for variant = 1:3
        started = tic;
        if variant == 2
            counts = dictionary(words,zero);
        else
            counts = dictionary(words([]),zero([]));
        end
        counts = update(counts,words,one,zero,variant);
        seconds = toc(started);
        assert(sum(double(values(counts))) == n);
        expected = floor(n/cardinality)+((0:min(n,cardinality)-1) < mod(n,cardinality));
        assert(isequal(double(values(counts))',expected));
        fprintf('type=%d variant=%d n=%d seconds=%.9f entries=%d\n',numeric,variant,n,seconds,numEntries(counts));
    end
end
end
function counts = update(counts,words,one,zero,variant)
for pos = 1:numel(words)
    key = words(pos);
    if variant == 1
        counts(key) = lookup(counts,key,'FallbackValue',zero)+one;
    elseif variant == 2
        counts(key) = counts(key)+one;
    elseif isKey(counts,key)
        counts(key) = counts(key)+one;
    else
        counts(key) = one;
    end
end
end
