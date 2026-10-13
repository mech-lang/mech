% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% Additive correction: prescribed variant counts use case-sensitive regexp.
function regexredux_optimized()
fid = fopen('/dev/fd/3','rb');
raw = fread(fid,Inf,'*char')';
fclose(fid);
original_length = numel(raw);
dna = regexprep(raw,'>[^\n]*\n|\n','');
clear raw;
patterns = ["agggtaaa|tttaccct" "[cgt]gggtaaa|tttaccc[acg]" "a[act]ggtaaa|tttacc[agt]t" "ag[act]gtaaa|tttac[agt]ct" "agg[act]taaa|ttta[agt]cct" "aggg[acg]aaa|ttt[cgt]ccct" "agggt[cgt]aa|tt[acg]accct" "agggta[cgt]a|t[acg]taccct" "agggtaa[cgt]|[acg]ttaccct"];
for j = 1:numel(patterns)
    starts = regexp(dna,patterns(j),'start');
    fprintf('%s %d\n',patterns(j),numel(starts));
end
clean_length = numel(dna);
substitutions = ["tHa[Nt]" "<4>"; "aND|caN|Ha[DS]|WaS" "<3>"; "a[NSt]|BY" "<2>"; "<[^>]*>" "|"; "\|[^|][^|]*\|" "-"];
for j = 1:size(substitutions,1)
    dna = regexprep(dna,substitutions(j,1),substitutions(j,2));
end
fprintf('\n%d\n%d\n%d\n',original_length,clean_length,numel(dna));
end
