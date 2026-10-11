addpath('/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/matlab-comparison/20261010/sources');
fprintf('AUDIT THREADS %d JVM %d\n',maxNumCompThreads,usejava('jvm'));
for audit_run = 1:3
fprintf('AUDIT BEGIN nbody %d\n',audit_run);
audit_start = tic;
nbody_optimized(50000000);
audit_elapsed = toc(audit_start);
fprintf('AUDIT END nbody %d %.17g\n',audit_run,audit_elapsed);
end
for audit_run = 1:3
fprintf('AUDIT BEGIN spectralnorm %d\n',audit_run);
audit_start = tic;
spectralnorm_optimized(5500);
audit_elapsed = toc(audit_start);
fprintf('AUDIT END spectralnorm %d %.17g\n',audit_run,audit_elapsed);
end
