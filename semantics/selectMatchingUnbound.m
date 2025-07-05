function vals = selectMatchingUnbound(symtab,identifier)

    % Select all identifiers in the table
    identifiers_mask = strcmp([symtab(:,3)],'$');
    all_identifiers = symtab(identifiers_mask,2);

    matching_inds = find(ismember(all_identifiers,identifier));
    unbound_inds = find(strcmp(symtab(identifiers_mask,5),'Unbound'));

    % Get matches and unbound
    matching_unbound_inds = intersect(unbound_inds,matching_inds);
    
    % Get vals for inds
    vals = [symtab{identifiers_mask,1}];
    vals = vals(matching_unbound_inds);
    
end