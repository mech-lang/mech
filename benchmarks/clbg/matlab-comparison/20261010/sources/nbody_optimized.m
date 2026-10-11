% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% All application work is here; standard MATLAB operations provide library support.
function nbody_optimized(n)
b = [0 0 0 0 0 0 1;
     4.84143144246472090 -1.16032004402742839 -0.103622044471123109 0.00166007664274403694 0.00769901118419740425 -0.0000690460016972063023 0.000954791938424326609;
     8.34336671824457987 4.12479856412430479 -0.403523417114321381 -0.00276742510726862411 0.00499852801234917238 0.0000230417297573763929 0.000285885980666130812;
     12.8943695621391310 -15.1111514016986312 -0.223307578892655734 0.00296460137564761618 0.00237847173959480950 -0.0000296589568540237556 0.0000436624404335156298;
     15.3796971148509165 -25.9193146099879641 0.179258772950371181 0.00268067772490389322 0.00162824170038242295 -0.0000951592254519715870 0.0000515138902046611451];
b(:,4:6) = b(:,4:6)*365.24;
b(:,7) = b(:,7)*(4*pi*pi);
b(1,4:6) = -sum(b(:,4:6).*b(:,7),1)/b(1,7);
fprintf('%.9f\n',energy(b));
x = b(:,1);
y = b(:,2);
z = b(:,3);
vx = b(:,4);
vy = b(:,5);
vz = b(:,6);
mass = b(:,7);
body_count = numel(x);
for step = 1:n
    for i = 1:body_count-1
        xi = x(i);
        yi = y(i);
        zi = z(i);
        vxi = vx(i);
        vyi = vy(i);
        vzi = vz(i);
        mi = mass(i);
        for j = i+1:body_count
            dx = xi-x(j);
            dy = yi-y(j);
            dz = zi-z(j);
            squared = dx*dx+dy*dy+dz*dz;
            magnitude = 0.01/(squared*sqrt(squared));
            mj = mass(j)*magnitude;
            vxi = vxi-dx*mj;
            vyi = vyi-dy*mj;
            vzi = vzi-dz*mj;
            scaled_i = mi*magnitude;
            vx(j) = vx(j)+dx*scaled_i;
            vy(j) = vy(j)+dy*scaled_i;
            vz(j) = vz(j)+dz*scaled_i;
        end
        vx(i) = vxi;
        vy(i) = vyi;
        vz(i) = vzi;
    end
    for i = 1:body_count
        x(i) = x(i)+0.01*vx(i);
        y(i) = y(i)+0.01*vy(i);
        z(i) = z(i)+0.01*vz(i);
    end
end
fprintf('%.9f\n',energy([x y z vx vy vz mass]));
end
function e = energy(b)
pairs = nchoosek(1:size(b,1),2);
delta = b(pairs(:,1),1:3)-b(pairs(:,2),1:3);
e = sum(b(:,7).*sum(b(:,4:6).^2,2))/2-sum(b(pairs(:,1),7).*b(pairs(:,2),7)./sqrt(sum(delta.^2,2)));
end
