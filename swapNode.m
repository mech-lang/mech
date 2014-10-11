function out_tree = swapNode(in_tree,node_ind)

    out_tree = in_tree;
    parent_ind = out_tree.getparent(node_ind);
    parent_val = out_tree.Node(parent_ind);
    
    % Promote the node to parent
    node_val = out_tree.Node(node_ind);
    out_tree.Node(parent_ind) = node_val;
    
    % Set old node to parent val
    out_tree.Node(node_ind) = parent_val;
    
end