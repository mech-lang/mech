function CAM = setMemoryContent(value,tag,CAM)

    ind = findIndex(tag,CAM);
    CAM(ind,2) = value;
    
end