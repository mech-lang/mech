%--------------------------------------------------------------------------
% Authors: Corey Montella
% Date Created:  10/12/2014
% Last Modified: 11/21/2014
% 
% Description: 
%  
% Parses math expressions
%
% INPUT:
%   
%   tokens - list of tokens to be parsed     
%
% OUTPUT:
%
%   unused_tokens - list of tokens not consumed in the math expression 
%                   parse
%   parse_tree - parse tree of the input token stream
%
% Changelog:
%
% 11/21/2014 - CIM - Added token identification for new / and - tokens
%                  - Changed urnary operator token to p, to conform to new
%                    lexer spec
% 10/16/2014 - CIM - Modified so that the full token is added to the tree
% 10/12/2014 - CIM - Created
%--------------------------------------------------------------------------

% Entry function to the parser, start parsing a program.
function [good,unused_tokens,parse_tree] = mathParser(tokens)

    [unused_tokens,parse_tree] = expression(tokens);

    if isempty(parse_tree.Node{1})
        good = 0;
    else
        good = 1;
    end
    
end

function [tokens,t] = expression(tokens)
    %disp('E')

    [tokens,t] = term(tokens);

    next = getNextToken(tokens);

    % Optional repeated add/sub operator
    while strcmp(next{2},'+') 
        op = next;
        tokens = consume(tokens);
        [tokens,t1] = term(tokens);
        t = makeNode(op,t,t1);
        next = getNextToken(tokens);
    end

    return
   
end

function [tokens,t] = term(tokens)
    %disp('T')   

    [tokens,t] = factor(tokens);

    next = getNextToken(tokens);

    % Optional repeated mul/div operator
    while strcmp(next{2},'*')
        op = next;
        tokens = consume(tokens);
        [tokens,t1] = factor(tokens);
        t = makeNode(op,t,t1);
        next = getNextToken(tokens);
    end

    return
   
end

function [tokens,t] = factor(tokens)
    %disp('F')
    
    [tokens,t] = P(tokens);

    next = getNextToken(tokens);
    
    % Optional single exponentiation operator
    if next{2} == '^'
        tokens = consume(tokens);
        [tokens,t1] = factor(tokens);
        t = makeNode(next,t,t1);
        return 
    else
        return
    end
    
end

function [tokens,t] = P(tokens)
    %disp('P')
    
    next = getNextToken(tokens);

    % Next token is a leaf
    if  intersect(next{2},'$.#')
        t = tree(next);
        tokens = consume(tokens);  
        return
        
    % Next token is a function identifier
    elseif next{2} == '@'
        t = tree(next);
        tokens = consume(tokens);  
        tokens = tokenExpect(tokens,'(');
        [tokens,t1] = expression(tokens);
        t = makeNode(t,t1);
        
        % Parse a list of expressions
        next = getNextToken(tokens);
        while next{2} == ','
            tokens = consume(tokens);
            [tokens,t1] = expression(tokens);
            next = getNextToken(tokens);
            t = makeNode(t,t1);
        end
        
        % Finish off the function identifier
        tokens = tokenExpect(tokens,')');

        return
        
    % Next token denotes a parenthetical expression
    elseif next{2} == '('
        tokens = consume(tokens);
        [tokens,t] = expression(tokens);       
        tokens = tokenExpect(tokens,')');
        return   

    % Next token is a urnary operator
    elseif next{2} == '-'
        tokens = consume(tokens);
        [tokens,t] = factor(tokens);
        t = makeNode(next,t);
        
    else
        t = tree();
    end
    
end

