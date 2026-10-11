function work_probe()
report = struct;
for profile = ["compact" "optimized"]
    constructor = str2func("make_"+profile);
    counts = zeros(3,5);
    row = 0;
    for depth = [4 8 12]
        tree = constructor(depth);
        stats = allocation_identity(tree);
        expected = 2^(depth+1)-1;
        assert(all(stats(1:3) == expected));
        assert(stats(4) == 2^depth && stats(5) == 1);
        row = row+1;
        counts(row,:) = stats;
        clear tree;
    end
    report.(profile) = counts;
end
fid = fopen('allocation-identity.json','w');
fprintf(fid,'%s\n',jsonencode(report));
fclose(fid);
end
function t = make_compact(depth)
t = cell(1,2);
if depth > 0
    t{1} = make_compact(depth-1);
    t{2} = make_compact(depth-1);
end
end
function count = walk_compact(t)
count = 1;
if ~isempty(t{1})
    count = count+walk_compact(t{1})+walk_compact(t{2});
end
end

function t = make_optimized(depth)
if depth >= 2
    left = {make_optimized(depth-2),make_optimized(depth-2)};
    right = {make_optimized(depth-2),make_optimized(depth-2)};
    t = {left,right};
elseif depth == 1
    t = {cell(1,2),cell(1,2)};
else
    t = cell(1,2);
end
end
function count = walk_optimized(t)
count = 1;
if ~isempty(t{1})
    count = count+walk_optimized(t{1})+walk_optimized(t{2});
end
end
