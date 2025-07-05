function out_tree = promoteNode(in_tree,node_ind)

    out_tree = in_tree;
    parent_ind = out_tree.getparent(node_ind);
    
    % Promote the node to parent
    node_val = out_tree.Node(node_ind);
    out_tree.Node(parent_ind) = node_val;
    
    % Remove the old node
    out_tree = out_tree.removenode(node_ind);

end