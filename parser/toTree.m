function t = toTree(adj_mat)

    % Initialize empty tree object
    t = tree();

    % Get the right parents from the adj mat
    ind = find(adj_mat.mat');
    [I,~] = ind2sub(size(adj_mat.mat),ind);
    Parent = [0 I']';
    
    
    % Build the new tree
    t.Node = adj_mat.nodes;
    t.Parent = Parent;

end