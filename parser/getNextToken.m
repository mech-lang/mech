function next = getNextToken(tokens)

    if ~isempty(tokens)
        next = tokens{1};
    else
        next = {[] []};
    end
    
end