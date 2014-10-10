% Each entry in PE has the following fields
%   - 

function PE = initializePE(symtab)

    PE = cell(size(symtab,1),4);
    
    for i = 1:size(symtab,1)
        
        row = symtab(i,:);
        
        % Copy the tag
        PE{i,1} = row{1};
        
        % Set the operation
        switch row{3}
            case '$'
                PE{i,2} = 'Assignment';
            case '+'
                PE{i,2} = row{2};
            case '*'
                PE{i,2} = row{2};
            otherwise
                PE{i,2} = 'Constant';
        end
        
        % Set the input memory locations
        if strcmp(row{5},'Terminal');
            PE{i,3} = row{1};
        else
            PE{i,3} = row{5};  
        end

        % Set the input status flags
        if strcmp(row{5},'Terminal');
            PE{i,4} = 1;
        else
            PE{i,4} = cell(size(row{5}));  
        end
        
    end
    
end