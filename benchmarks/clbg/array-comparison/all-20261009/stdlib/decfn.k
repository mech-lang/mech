decode:{[width;key]
 weights:|(*\1,(width-1)#4)
 d:4!_(key%weights)
 "ACGT"@d}
`0:decode[1;3]
`0:decode[2;3]
`0:decode[2;12]
`0:decode[2;0]
