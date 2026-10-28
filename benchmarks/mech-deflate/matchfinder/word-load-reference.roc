bytes : List(U8)
bytes = [1, 2, 3, 4, 5, 6, 7, 8]

expect (U32.from_le_bytes(bytes, 0) ?? 0) == 0x04030201
expect (U64.from_le_bytes(bytes, 0) ?? 0) == 0x0807060504030201
expect (U32.from_le_bytes(bytes, 4) ?? 0) == 0x08070605
expect (U32.from_le_bytes(bytes, 5) ?? 0) == 0
expect (U64.from_le_bytes(bytes, 1) ?? 0) == 0
expect (U32.from_le_bytes(bytes, 0xFFFFFFFFFFFFFFFF) ?? 0) == 0
expect (U64.from_le_bytes(bytes, 0xFFFFFFFFFFFFFFFF) ?? 0) == 0
expect (U64.from_le_bytes([], 0) ?? 0) == 0

expect (List.get(bytes, 0) ?? 0) == 1
expect (List.get(bytes, 0xFFFFFFFFFFFFFFFF) ?? 0) == 0
