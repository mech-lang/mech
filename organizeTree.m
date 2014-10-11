function out_tree = organizeTree(in_tree)

    out_tree = in_tree;
    
    dfs_iterator = in_tree.depthfirstiterator;
    dfs_iterator = dfs_iterator(end:-1:3);
    
    
    for i = dfs_iterator
            
        % Look at the first node node
        node_val = out_tree.Node(i);

        disp(out_tree.tostring);
        
        % Get precedence level
        p = getPrecedence(node_val);
        
        % Remove node if has no precedence
        if p < 0
            out_tree = out_tree.removenode(i);
            continue;            
        end
        
        % Look at its parent
        parent_ind = out_tree.getparent(i);
        parent_val = out_tree.Node(parent_ind);

        % Get precedence level
        pp = getPrecedence(parent_val);
        
        % If the parent has no precedence, promote the child
        if pp < 0
           out_tree = promoteNode(out_tree,i);
        % If there is precedence, decide whether to swap
        elseif p < pp
            out_tree = swapNode(out_tree,i);
%         elseif p == pp
%             out_tree = swapNode(out_tree,parent_ind);
%             keyboard
%             out_tree = promoteNode(out_tree,i);
%             keyboard
        end
        
    end
    


end


function precedence = getPrecedence(value)


    level1 = {'+'};
    level2 = {'*'};
    level3 = {'$' '#'};
    
    if any(ismember(level1,value))
        precedence = 1;
    elseif any(ismember(level2,value))
        precedence = 2;
    elseif any(ismember(level3,value))
        precedence = 3;
    else
        precedence = -1;
    end
    


end