% Redirects parent nodes to a new child node
function symtab = redirectParents(symtab,parents,new_child,matches)

    for i = parents
        
        % Get children of new parent
        ind = findIndex(i,symtab);
        children = symtab{ind,5};
        
        % Get the index of the child we want to redirect
        [~,matching_ind] = intersect(children,matches);
        
        % Redirect matching child to the new child
        children(matching_ind) = new_child;
        symtab{ind,5} = children;
        
    end
    
end