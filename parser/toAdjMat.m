function adj_mat = toAdjMat(t)

    n = length(t.Node);
    
    mat = zeros(n,n);
    
    for i = 2:n
        
        j = t.Parent(i);
        
        mat(i,j) = 1;
        
    end

    adj_mat.nodes = t.Node;
    adj_mat.mat = mat;
    
end