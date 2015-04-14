function [output, PE, CAM] = executionTest(test_file)

    [PE, CAM] = generationTest(test_file);
    tic
    [output, PE, CAM] = executeNetwork(PE,CAM);
    toc
    disp('Execution Test Passed');
    
end