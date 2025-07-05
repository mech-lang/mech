function PE = setStatusFlags(node_index,PE)
    
    % Take out the node from the PE
    node = PE(node_index,:);
    node_tag = getNodeTag(node);

    % Get the tags for the node's parents
    parents = getNodeParents(node);
    
    % Find the indices for the node's parents
    parent_inds = findIndex(parents,PE);
    
    % Output nodes don't set any status flags
    if any(parent_inds == 0)
       return; 
    end    
    
    % Take the node's parents out of the PE
    parent_nodes = PE(parent_inds,:);
    
    % Set the correct flags for the parent nodes
    for i = 1:size(parent_nodes,1)
        
        parent_node = parent_nodes(i,:);
        parent_children_nodes = getNodeChildren(parent_node);
        parent_status_flags = getNodeStatusFlags(parent_node);
        
        % Get the right index to set
        [~,flag_ind] = ismember(node_tag,parent_children_nodes);
        
        % Activate the flag
        parent_status_flags(flag_ind) = 1;
        
        % Put it back in the PE
        PE{parent_inds(i),5} = parent_status_flags;
        
    end
    
end