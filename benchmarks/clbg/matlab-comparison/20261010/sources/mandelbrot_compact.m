% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% Compact and optimized candidates; full required work, no native benchmark executor.
function mandelbrot_compact(n)
out = fopen('/dev/stdout','wb');
fprintf(out,'P4\n%d %d\n',n,n);
x = (0:n-1)*2/n-1.5;
for y = 0:n-1
    c = x+1i*(y*2/n-1);
    z = zeros(1,n);
    alive = true(1,n);
    for iteration = 1:50
        z = z.*z+c;
        alive = alive & real(z).^2+imag(z).^2 <= 4;
        z(~alive) = 0;
    end
    padded = [alive false(1,mod(-n,8))];
    fwrite(out,uint8([128 64 32 16 8 4 2 1]*reshape(padded,8,[])),'uint8');
end
fclose(out);
end
