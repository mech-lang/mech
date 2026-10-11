% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% Compact and optimized candidates; full required work, no native benchmark executor.
function mandelbrot_optimized(n)
out = fopen('/dev/stdout','wb');
fprintf(out,'P4\n%d %d\n',n,n);
width = ceil(n/8);
for first = 0:32:n-1
    last = min(first+31,n-1);
    cr = repmat((0:n-1)*2/n-1.5,last-first+1,1);
    ci = repmat((first:last)'*2/n-1,1,n);
    zr = zeros(size(cr));
    zi = zeros(size(cr));
    live = (1:numel(cr))';
    for iteration = 1:50
        r = zr(live);
        i = zi(live);
        zr(live) = r.*r-i.*i+cr(live);
        zi(live) = 2*r.*i+ci(live);
        live = live(zr(live).^2+zi(live).^2 <= 4);
        if isempty(live)
            break;
        end
    end
    alive = false(size(cr));
    alive(live) = true;
    bits = [alive false(size(cr,1),width*8-n)]';
    bytes = uint8([128 64 32 16 8 4 2 1]*reshape(bits,8,[]));
    fwrite(out,bytes,'uint8');
end
fclose(out);
end
