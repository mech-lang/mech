function [good,tokens,acc] = peekTest(tokens,acc,varargin)

    acc.blanks = [acc.blanks ' '];

    n = nargin - 2;
    test_result = zeros(1,n);
    
    semantics = tokens{2};
    if isempty(semantics)
        peek = ' ';
    else
        peek = semantics(1);
    end
        
    for i = 1:n

        % Test the peek again the expected symbol list
        fprintf(acc.blanks);
        fprintf(['peek test (%s:%s)     ' genWhite(40-length(acc.blanks)) '%s\n'],peek,varargin{i},tokens{2});
        test_result(i) = peek == varargin{i};
    end

    % If any of the peek symbols meet an expected symbol, continue
    good = any(test_result);
    
    acc.parse_tree = acc.parse_tree.addnode(1,'e');

end