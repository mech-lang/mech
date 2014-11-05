function [good,ast] = parserTest(test_file)

    tokens = lexerTest(test_file);
    [good,ast] = parser(tokens);
    
    if good
        disp('Parse success!');
    else
        error('Failed to parse');
    end
    
end