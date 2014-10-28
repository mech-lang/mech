function tokens = lexerTest(test_file)

    src = loadSource(test_file);
    tokens = lexer(src);   
    disp('Lexer Test Passed');
    
end