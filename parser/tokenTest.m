% General function for testing a token against a test token
function [good,tokens,ast] = tokenTest(tokens,test,value)

    next = getNextToken(tokens);
    
    if isempty(next)
        next = {'e','e'};
    end
    
    % Perform the token test
    good = next{2} == test;

    % Perform the value test
    if nargin == 3
        good = strcmp(next{1},value);
    end
    
    % Consume tokens
    tokens = consume(tokens);
        
    ast = tree(next);
    
end