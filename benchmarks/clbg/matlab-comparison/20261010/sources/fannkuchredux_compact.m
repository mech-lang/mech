% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% All application work is here; standard MATLAB operations provide library support.
function fannkuchredux_compact(n)
p = 1:n;
count = zeros(1,n);
r = n;
rank = 0;
checksum = 0;
maximum = 0;
while true
    while r > 1
        count(r) = r;
        r = r-1;
    end
    flips = 0;
    copy = p;
    while copy(1) ~= 1
        k = copy(1);
        copy(1:k) = copy(k:-1:1);
        flips = flips+1;
    end
    checksum = checksum+(-1)^rank*flips;
    maximum = max(maximum,flips);
    rank = rank+1;
    while true
        if r == n
            fprintf('%d\nPfannkuchen(%d) = %d\n',checksum,n,maximum);
            return;
        end
        p(1:r+1) = p([2:r+1 1]);
        count(r+1) = count(r+1)-1;
        if count(r+1) > 0
            break;
        end
        r = r+1;
    end
end
end
