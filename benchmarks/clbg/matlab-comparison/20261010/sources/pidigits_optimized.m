% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% Compact and optimized candidates; full required work, no native benchmark executor.
function pidigits_optimized(n)
q = java.math.BigInteger.ONE;
r = java.math.BigInteger.ZERO;
t = q;
k = int64(1);
three = java.math.BigInteger.valueOf(int64(3));
four = java.math.BigInteger.valueOf(int64(4));
ten = java.math.BigInteger.TEN;
line = repmat(' ',1,10);
emitted = 0;
while emitted < n
    a = q.multiply(three).add(r).divide(t);
    b = q.multiply(four).add(r).divide(t);
    if a.equals(b)
        line(mod(emitted,10)+1) = char(double(a.intValue())+48);
        emitted = emitted+1;
        r = r.subtract(t.multiply(a)).multiply(ten);
        q = q.multiply(ten);
        if mod(emitted,10) == 0 || emitted == n
            fprintf('%s\t:%d\n',line,emitted);
            line(:) = ' ';
        end
    else
        odd = java.math.BigInteger.valueOf(2*k+1);
        r = q.multiply(java.math.BigInteger.valueOf(4*k+2)).add(r.multiply(odd));
        q = q.multiply(java.math.BigInteger.valueOf(k));
        t = t.multiply(odd);
        k = k+1;
    end
end
end
