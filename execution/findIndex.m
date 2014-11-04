function inds = findIndex(tags,Arry)

    search = [Arry{:,1}];
    [~,inds] = ismember(tags,search);

end