% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% All application work is here; standard MATLAB operations provide library support.
function fannkuchredux_optimized(n)
p = 1:n;
count = zeros(1,n);
r = n;
sign = 1;
checksum = 0;
maximum = 0;
while true
    while r > 1
        count(r) = r;
        r = r-1;
    end
    flips = 0;
    if p(1) ~= 1
        copy = p;
        k = copy(1);
        while k ~= 1
            lo = 1;
            hi = k;
            while lo < hi
                temp = copy(lo);
                copy(lo) = copy(hi);
                copy(hi) = temp;
                lo = lo+1;
                hi = hi-1;
            end
            flips = flips+1;
            k = copy(1);
        end
    end
    checksum = checksum+sign*flips;
    maximum = max(maximum,flips);
    sign = -sign;
    while true
        if r == n
            fprintf('%d\nPfannkuchen(%d) = %d\n',checksum,n,maximum);
            return;
        end
        first = p(1);
        for pos = 1:r
            p(pos) = p(pos+1);
        end
        p(r+1) = first;
        count(r+1) = count(r+1)-1;
        if count(r+1) > 0
            break;
        end
        r = r+1;
    end
end
end
