function PE = resetStatusFlags(node_tag,PE)

    ind = findIndex(node_tag,PE);
    status_flags = PE{ind,5};
    PE{ind,5} = zeros(size(status_flags));

end