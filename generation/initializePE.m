%--------------------------------------------------------------------------
% Authors: Corey Montella
% Date Created:  10/16/2014
% Last Modified: 11/20/2014
% 
% Description: 
%  
% Each entry in PE has the following fields
%   - Tag
%   - Processor operation
%   - Parent nodes
%   - Input memory locations in CAM
%   - Input status flags
%       - 1 indicates ready to fire
%       - 0 indicates waiting for inputs
%
% INPUT:
%   
%   symtab - symtbol table generated from generateSymbolTable
%
% OUTPUT:
%
%   PE - Processing Element array
%      [1] - Tag
%      [2] - Processor operation
%      [3] - Parent node addresses
%      [4] - Input memory addresses in CAM
%      [5] - Status flags
%
% Changelog:
% 
% 11/20/2014 - CIM - Added array [] to list of operations
% 10/16/2014 - CIM - Created
%--------------------------------------------------------------------------

function PE = initializePE(symtab)

    PE = cell(size(symtab,1),5);

    for i = 1:size(symtab,1)
        
        row = symtab(i,:);
        
        % Copy the tag
        PE{i,1} = row{1};
        
        % Set the operation
        switch row{3}
            case '$'
                PE{i,2} = 'Assignment';
            case '@'
                PE{i,2} = row{2};
            case '+'
                if row{2} == '+'
                    PE{i,2} = 'plus';
                elseif row{2} == '-'
                    PE{i,2} = 'minus';
                end
            case '*'
                if row{2} == '*'
                    PE{i,2} = 'times';
                elseif row{2} == '/'
                    PE{i,2} = 'rdivide';
                end
            case '^'
                PE{i,2} = 'power';
            case 'p'
                PE{i,2} = 'uminus';
            case '['
                PE{i,2} = row{2};
            otherwise
                PE{i,2} = 'Constant';
        end
        
        % Set the output nodes
        if strcmp(row{4},'Output')
            PE{i,3} = 0;
            PE{i,2} = 'Output';
        else
            PE{i,3} = row{4};
        end
        
        % Set the input memory locations
        if strcmp(row{5},'Terminal');
            PE{i,4} = row{1};
        else
            PE{i,4} = row{5};  
        end

        % Set the input status flags
        if strcmp(row{5},'Terminal');
            PE{i,5} = 1;
        else
            PE{i,5} = zeros(1,size(row{5},2));  
        end
                
    end

end