% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% Compact and optimized candidates; full required work, no native benchmark executor.
function pidigits_compact(n)
q = java.math.BigInteger.ONE;
r = java.math.BigInteger.ZERO;
t = q;
k = 1;
emitted = 0;
line = '';
while emitted < n
    a = q.multiply(big(3)).add(r).divide(t);
    b = q.multiply(big(4)).add(r).divide(t);
    if a.equals(b)
        line = [line char(a.toString())];
        emitted = emitted+1;
        r = r.subtract(t.multiply(a)).multiply(big(10));
        q = q.multiply(big(10));
        if mod(emitted,10) == 0 || emitted == n
            fprintf('%-10s\t:%d\n',line,emitted);
            line = '';
        end
    else
        r = q.multiply(big(4*k+2)).add(r.multiply(big(2*k+1)));
        q = q.multiply(big(k));
        t = t.multiply(big(2*k+1));
        k = k+1;
    end
end
end
function value = big(x)
value = java.math.BigInteger.valueOf(int64(x));
end
