function [output,PE,CAM] = executeNetwork(PE,CAM)

    % Get output nodes
    output_nodes_mask = strcmp(PE(:,2),'Output');
    output_nodes_inds = 1:size(PE,1);
    output_nodes_inds = output_nodes_inds(output_nodes_mask);
    output_nodes_tags = [PE{output_nodes_inds,4}];
    
    output = getMemoryContent(output_nodes_tags,CAM);

    while any(cellfun(@isempty,output))
        
        % Fire all the ready nodes
        for i = 1:size(PE,1)

            node = PE(i,:);

            status_flags = getNodeStatusFlags(node);
            
            if all(status_flags)
                
                [PE, CAM] = fireNode(i,PE,CAM);
            
            end
            
        end
        
        output = getMemoryContent(output_nodes_tags,CAM);
                
    end
    
end