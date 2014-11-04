function symtab = semanticsTest(test_file)

    [~,ast] = parserTest(test_file);
    symtab = semanticAnalysis(ast);
    disp('Semantics Test Passed');
    
end