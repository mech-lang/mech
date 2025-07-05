function [token_src_rep] = tokenize(token_src,reg_exp,token_rep)

    % Token Src is a cell array of strings, each one either being a token,
    % which is designared by the fact that it is a cell sized 1x2, or
    % source code
    token_src_rep = cell(0,0);
    
    % Go through each cell
    for i = 1:length(token_src)
        
        current_cell = token_src(i);
        
        % If the current cell is a token, just add it to the output source
        % and continue
        if iscell(current_cell{1})
            token_src_rep = [token_src_rep current_cell];
            continue; 
        end
            
        % If the current cell is not a token, perform a reg exp on it using
        % the supplied reg exp
        [matched_strings,split_string] = regexp(current_cell,reg_exp,'match','split');
        if isempty(matched_strings{1})
            token_src_rep = [token_src_rep current_cell];
            continue;
        end
        
        % Form tokens from the matched regexps
        tokens = formTokens(matched_strings,token_rep);
        
        % Zip the tokens and the split string into one single cell array
        token_src_line = zipCells(tokens,split_string);
        
        % Add the new cells into the replaced 
        token_src_rep = [token_src_rep token_src_line];
       
    end


end