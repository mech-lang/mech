function src = loadSource(source_file)
    
    src = [];

    fid = fopen(source_file);

    line = fgetl(fid);
    while ischar(line)
        src{length(src)+1} = line;
        line = fgetl(fid);
    end
   
    fclose(fid);

end