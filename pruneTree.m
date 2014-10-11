function pruned_tree = pruneTree(in_tree,string)

    % Find instances of the node
    inds = find(strcmp(in_tree.Node,string));

    % Check to see if the nodes to be removed have any children
    if ~isempty(intersect(in_tree.Parent,inds))
        while ~isempty(inds)
            in_tree = in_tree.removenode(inds(1));            
            inds = find(strcmp(in_tree.Node,string));
        end
        
        pruned_tree = in_tree;
        return;
    end    
    
    % Convert to adj mat representation
    adj_mat = toAdjMat(in_tree);

    % Remove nodes from node list
    adj_mat.nodes(inds) = [];

    % Remove rows and cols in the adj mat
    adj_mat.mat(inds,:) = [];
    adj_mat.mat(:,inds) = [];
    
    % Convert back to tree representation
    pruned_tree = toTree(adj_mat);
    
    
end