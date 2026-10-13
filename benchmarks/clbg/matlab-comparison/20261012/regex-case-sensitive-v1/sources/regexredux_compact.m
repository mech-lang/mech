% Local MATLAB implementation of the Computer Language Benchmarks Game.
% https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/
% Additive correction: prescribed variant counts use case-sensitive regexp.
function regexredux_compact()
fid = fopen('/dev/fd/3','rb');
raw = fread(fid,Inf,'*char')';
fclose(fid);
dna = regexprep(raw,'>[^\n]*\n|\n','');
patterns = ["agggtaaa|tttaccct" "[cgt]gggtaaa|tttaccc[acg]" "a[act]ggtaaa|tttacc[agt]t" "ag[act]gtaaa|tttac[agt]ct" "agg[act]taaa|ttta[agt]cct" "aggg[acg]aaa|ttt[cgt]ccct" "agggt[cgt]aa|tt[acg]accct" "agggta[cgt]a|t[acg]taccct" "agggtaa[cgt]|[acg]ttaccct"];
for pattern = patterns
    fprintf('%s %d\n',pattern,numel(regexp(dna,pattern,'start')));
end
changed = dna;
substitutions = ["tHa[Nt]" "<4>"; "aND|caN|Ha[DS]|WaS" "<3>"; "a[NSt]|BY" "<2>"; "<[^>]*>" "|"; "\|[^|][^|]*\|" "-"];
for j = 1:size(substitutions,1)
    changed = regexprep(changed,substitutions(j,1),substitutions(j,2));
end
fprintf('\n%d\n%d\n%d\n',numel(raw),numel(dna),numel(changed));
end
