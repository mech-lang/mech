function [PE, CAM] = generationTest(test_file)


symtab = semanticsTest(test_file);
    [PE, CAM] = initializeNetwork(symtab);
    disp('Code Generation Test Passed');

end