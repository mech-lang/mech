%--------------------------------------------------------------------------
% Authors: Corey Montella
% Date Created:  10/16/2014
% Last Modified: 10/16/2014
% 
% Description: 
%  
% Takes AST and generates a dataflow network.
%
% INPUT:
%   
%   ast  - abstract syntax tree  
%
% OUTPUT:
%
%   symtab - table of symbols, with the following columns
%      [1] - tag
%      [2] - symbol identifier
%      [3] - symbol type
%      [4] - parent nodes
%      [5] - children nodes
%
% Changelog:
% 
% 10/16/2014 - CIM - Created
%--------------------------------------------------------------------------


function symtab = semanticAnalysis(ast)

    symtab = [];

    % Generate symbol table for each AST
    for i = 1:length(ast)
        
        n = size(symtab,1);
        
        % Get the symtab of the current tree
        thistab = generateSymbolTable(ast(i));
        
        % Modify the edges of the current table
        thistab = incrementEdges(thistab,n);
       
        % Combine with the symbol table
        symtab = [symtab; thistab];
        
    end
    
    % Go through the symbol table and connect unbound variables
    % (matches 'output' with 'unbound')
        
    for token = 1:size(symtab,1)
        
        row_ind = findIndex(token,symtab);
                
        if row_ind == 0
            continue;
        end
        
        identifier = symtab{row_ind,2};
        symbol = symtab{row_ind,3};
        parent = symtab{row_ind,4};
        children = symtab{row_ind,5};

        % If the current line is an output identifier, search for all
        % unbound counterparts
        if strcmp(symbol,'$')  && strcmp(symtab(parent,4),'Output')

            % Select all matching identifiers that are unbound
            matches = selectMatchingUnbound(symtab,identifier);
            
            % If there are matches, redirect the graph
            if ~isempty(matches)
                
                % Give the current row new parents
                ind = findIndex(matches,symtab);
                new_parents = [symtab{ind,4}];
                symtab{row_ind,4} = new_parents;
                    
                % Redirect new parents to current row
                symtab = redirectParents(symtab,new_parents,token,matches);

                % Remove redundant table entries
                symtab = removeRows(symtab,ind);
                
                % Remove output reference to current row
                childmask = symtab{parent,5} == token;
                parent_children = symtab{parent,5};
                parent_children(childmask) = [];
                symtab{parent,5} = parent_children;
                
            end

        end

    end
    
end