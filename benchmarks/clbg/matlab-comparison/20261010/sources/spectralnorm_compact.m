% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% Compact and optimized candidates; full required work, no native benchmark executor.
function spectralnorm_compact(n)
u = ones(n,1);
for step = 1:10
    v = ata(u);
    u = ata(v);
end
fprintf('%.9f\n',sqrt(dot(u,v)/dot(v,v)));
end
function a = coefficient(i,j)
a = 1./((i+j).*(i+j+1)/2+i+1);
end
function w = multiply(v)
n = length(v);
w = zeros(n,1);
for i = 0:n-1
    w(i+1) = sum(coefficient(i,0:n-1).*v');
end
end
function w = transpose_multiply(v)
n = length(v);
w = zeros(n,1);
for i = 0:n-1
    w(i+1) = sum(coefficient(0:n-1,i).*v');
end
end
function w = ata(v)
w = transpose_multiply(multiply(v));
end
