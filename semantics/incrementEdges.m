function symtab = incrementEdges(symtab,n)

    for i = 1:size(symtab,1)
        
        % Update ID
        symtab{i,1} = symtab{i,1} + n;

        % Update parent
        if isnumeric(symtab{i,4})
            symtab{i,4} = symtab{i,4} + n;
        end

        % Update children
        if isnumeric(symtab{i,5})
            symtab{i,5} = symtab{i,5} + n;
        end
        
    end

end