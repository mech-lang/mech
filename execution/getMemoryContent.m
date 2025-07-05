function content = getMemoryContent(tag,CAM)

    ind = findIndex(tag,CAM);

    content = CAM(ind,2);
    
end