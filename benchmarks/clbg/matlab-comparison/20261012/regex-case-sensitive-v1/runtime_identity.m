function runtime_identity()
workers = maxNumCompThreads;
assert(strcmp(version('-release'),'2026b'));
assert(strcmp(computer('arch'),'maca64'));
assert(workers >= 1 && workers <= 8);
fprintf(2,'CLBG_MATLAB_VERSION=%s\nCLBG_MATLAB_WORKERS=%d\n',version,workers);
fprintf('MATLAB R2026b native arm64 <=8 compute verified\n');
end
