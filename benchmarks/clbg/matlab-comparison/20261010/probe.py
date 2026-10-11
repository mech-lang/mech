"""Generate untimed work probes using pinned application function bodies."""
import json,re
from pathlib import Path
D=Path(__file__).resolve().parent

def main():
    blocks=[]
    for profile in ['compact','optimized']:
        source=(D/'sources'/('binarytrees_'+profile+'.m')).read_text()
        functions=source[source.index('function t = make_tree'):]
        functions=re.sub(r'\bmake_tree\b','make_'+profile,functions)
        functions=re.sub(r'\bcheck\b','walk_'+profile,functions)
        blocks.append(functions)
    body="""function work_probe()
report = struct;
for profile = ["compact" "optimized"]
    constructor = str2func("make_"+profile);
    counts = zeros(3,5);
    for depth = [4 8 12]
        tree = constructor(depth);
        stats = allocation_identity(tree);
        expected = 2^(depth+1)-1;
        assert(all(stats(1:3) == expected));
        assert(stats(4) == 2^depth && stats(5) == 1);
        counts(log2(expected+1)/4,:) = stats;
        clear tree;
    end
    report.(profile) = counts;
end
fid = fopen('allocation-identity.json','w');
fprintf(fid,'%s\\n',jsonencode(report));
fclose(fid);
end
"""
    # Explicit loop row avoids indexing by depth; all three arbitrary depths are checked.
    body=body.replace('    for depth = [4 8 12]','    row = 0;\n    for depth = [4 8 12]').replace('        counts(log2(expected+1)/4,:) = stats;','        row = row+1;\n        counts(row,:) = stats;')
    (D/'evidence/work_probe.m').write_text(body+'\n'.join(blocks))
    print('Generated identical tree constructors for an untimed generic allocation inventory.')
if __name__=='__main__':main()
