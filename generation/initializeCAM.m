%--------------------------------------------------------------------------
% Authors: Corey Montella
% Date Created:  10/16/2014
% Last Modified: 10/16/2014
% 
% Description: 
%  
% Each entry in the CAM has the following fields
%   - tag
%   - memory content
%
% INPUT:
%   
%   symtab - symtbol table generated from generateSymbolTable
%
% OUTPUT:
%
%   CAM - Content Addressable Memory
%      [1] - Tag
%      [2] - Memory Content
%
% Changelog:
% 
% 10/16/2014 - CIM - Created
%--------------------------------------------------------------------------

function CAM = initializeCAM(symtab)

    CAM = cell(size(symtab,1),2);

    for i = 1:size(symtab,1)
        row = symtab(i,:);
        
        % Copy the tag
        CAM{i,1} = row{1};
        
        % Initialize memory for node output
        if strcmp(row{4},'Output');
            CAM{i,2} = [];
        elseif strcmp(row{5},'Terminal');
            CAM{i,2} = row{2};
        else
            CAM{i,2} = [];  
        end               
    end

end