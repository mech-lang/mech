%--------------------------------------------------------------------------
% Authors: Corey Montella
% Date Created:  10/16/2014
% Last Modified: 04/13/2015
% 
% Description: 
% 
% Takes an abstraqct syntax tree and turns it into a table of relevent
% symbols
%
% INPUT:
%   
%   ast - Abstract syntax tree from parser
%
% OUTPUT:
%
%   Symtab - Symbol Table
%      [1] - Tag
%      [2] - Token Value
%      [3] - Token Type
%      [4] - Parent Tags
%      [5] - Child Tags
%
% Changelog:
% 
% 04/13/2015 - CIM - Fixed to match lexer tokens
% 10/16/2014 - CIM - Created
%--------------------------------------------------------------------------

function symtab = generateSymbolTable(ast)
    
    dfsi = ast.depthfirstiterator;
    dfsi = dfsi(end:-1:1);
    
    symtab = cell(length(dfsi),5);
    
    for i = dfsi
        
        node = ast.Node{i}
        symtab{i,1} = i;
        symtab(i,3) = node(2);
        
        % If the node is numeric, convert string to numeric and insert
        % value to symtab
        if strcmp(node{2},'#') || strcmp(node{2},'.')
            symtab{i,2} = str2num(node{1});
        % Otherwise, copy to the symtab
        else
            symtab(i,2) = node(1);
        end

        % Add parents to table
        parent = ast.getparent(i);
        if parent == 0
            parent = 'Output';
        end
        
        symtab(i,4) = {parent};
        
        % Add children to table
        children = ast.getchildren(i);
        if isempty(children) && node{2} == '$'
            children = 'Unbound';
        elseif isempty(children) && (node{2} == '#' || node{2} =='.')
            children = 'Terminal';
        end
        
        symtab(i,5) = {children};
        
    end

end