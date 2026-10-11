% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% Compact and optimized candidates; full required work, no native benchmark executor.
function spectralnorm_optimized(n)
u = ones(n,1);
for step = 1:10
    v = ata(u);
    u = ata(v);
end
fprintf('%.9f\n',sqrt(dot(u,v)/dot(v,v)));
end
function a = coefficient(i,j)
s = i+j;
a = 1./(s.*(s+1)/2+i+1);
end
function w = multiply(v)
n = length(v);
w = zeros(n,1);
j = 0:n-1;
for first = 1:64:n
    last = min(first+63,n);
    i = (first-1:last-1)';
    w(first:last) = coefficient(i,j)*v;
end
end
function w = transpose_multiply(v)
n = length(v);
w = zeros(n,1);
i = 0:n-1;
for first = 1:64:n
    last = min(first+63,n);
    j = (first-1:last-1)';
    w(first:last) = coefficient(i,j)*v;
end
end
function w = ata(v)
w = transpose_multiply(multiply(v));
end
