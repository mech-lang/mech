function [token_src] = zipCells(tokens,split_src)

    split_src = split_src{1};
    tokens_first = isempty(split_src{1});
    empty_mask = cellfun(@isempty,split_src);
    split_src(empty_mask) = [];
    
    % Allocate space for an output cell array
    token_src = cell(1,length(tokens)+length(split_src));
    
    % If the first cell in the split source is empty, start zipping with
    % the tokens, otherwise start zipping with the source
    if tokens_first
        first = tokens;
        second = split_src;
    else
        first = split_src;
        second = tokens;
    end
    
    % Perform the zip
    token_src(1:2:end) = first;
    token_src(2:2:end) = second;
    
end