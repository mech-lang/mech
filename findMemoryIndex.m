function inds = findMemoryIndex(tags,CAM)

    [~,~,inds] = intersect(tags,[CAM{:,1}],'stable');

end