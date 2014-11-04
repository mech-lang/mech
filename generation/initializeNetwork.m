function [PE, CAM] = initializeNetwork(symtab)

    % Allocate processing elements (PE) for each node
    PE = initializePE(symtab);

    % Initialize the Content Addressable Memory (CAM)
    CAM = initializeCAM(symtab);

    % Propagate constants and assignments
    % this is an optimization I can do later...

end