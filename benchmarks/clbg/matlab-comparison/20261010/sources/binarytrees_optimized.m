% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% All application work is here; standard MATLAB operations provide library support.
function binarytrees_optimized(n)
maximum = max(6,n);
stretch = make_tree(maximum+1);
fprintf('stretch tree of depth %d\t check: %d\n',maximum+1,check(stretch));
clear stretch;
long_lived = make_tree(maximum);
for depth = 4:2:maximum
    iterations = 2^(maximum-depth+4);
    total = 0;
    for iteration = 1:iterations
        tree = make_tree(depth);
        total = total+check(tree);
        clear tree;
    end
    fprintf('%d\t trees of depth %d\t check: %d\n',iterations,depth,total);
end
fprintf('long lived tree of depth %d\t check: %d\n',maximum,check(long_lived));
end
function t = make_tree(depth)
if depth >= 2
    left = {make_tree(depth-2),make_tree(depth-2)};
    right = {make_tree(depth-2),make_tree(depth-2)};
    t = {left,right};
elseif depth == 1
    t = {cell(1,2),cell(1,2)};
else
    t = cell(1,2);
end
end
function count = check(t)
count = 1;
if ~isempty(t{1})
    count = count+check(t{1})+check(t{2});
end
end
