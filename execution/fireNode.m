function [PE, CAM] = fireNode(node_index,PE,CAM)

    node = PE(node_index,:);
    node_tag = getNodeTag(node);
    
    if strcmp(node{2},'Constant');
        PE = setStatusFlags(node_index,PE);
        PE = resetStatusFlags(node_tag,PE);
        return;
    end
    
    node_children = getNodeChildren(node);

    % Fetch current memory contents
    content = getMemoryContent(node_children,CAM);

    % Execute operation on contents
    value = executeOperation(node,content);
    
    % Store result in CAM
    CAM = setMemoryContent({value},node_tag,CAM);
    
    % Set the status flags on the node's parents
    PE = setStatusFlags(node_index,PE);

    % Reset status flags on current node
    PE = resetStatusFlags(node_tag,PE);
    
end