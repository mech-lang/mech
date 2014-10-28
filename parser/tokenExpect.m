function tokens = tokenExpect(tokens,test_token)

    next = getNextToken(tokens);

    if next{2} == test_token
        tokens = consume(tokens);
    else
        error('Expected %s',test_token);
    end

end