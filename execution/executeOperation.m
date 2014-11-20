function value = executeOperation(node,content)

    op = node{2};
    
    % Assignment simply passes through the input
    if strcmp(op,'Assignment')
        value = content{1};
    % Output passes through the input as well
    elseif strcmp(op,'Output')
        value = content{1};
    % Anything else executes the specified operation on all inputs
    elseif strcmp(op,'[]')
        value = [content{:}];
    else
        value = feval(op,content{:});
    end
    
end