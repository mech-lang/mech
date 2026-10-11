"""Growing u64-to-u64 open-addressed map with ordinary entry updates."""

struct UIntMap(Movable):
    var keys: List[UInt64]
    var values: List[UInt64]
    var occupied: List[UInt8]
    var used: Int

    def __init__(out self):
        self.keys = List[UInt64](length=16, fill=0)
        self.values = List[UInt64](length=16, fill=0)
        self.occupied = List[UInt8](length=16, fill=0)
        self.used = 0

    @always_inline
    def slot(self, key: UInt64) -> Int:
        var mixed = (key ^ (key >> 32)) * 0x9E3779B97F4A7C15
        mixed ^= mixed >> 29
        var mask = len(self.keys) - 1
        var index = Int(mixed & UInt64(mask))
        var keys = self.keys.unsafe_ptr()
        var used = self.occupied.unsafe_ptr()
        while used[index] and keys[index] != key:
            index = (index + 1) & mask
        return index

    @always_inline
    def get(self, key: UInt64, default: UInt64 = 0) -> UInt64:
        var index = self.slot(key)
        return self.values.unsafe_ptr()[index] if self.occupied.unsafe_ptr()[index] else default

    @no_inline
    def grow(mut self):
        var old_keys = self.keys^
        var old_values = self.values^
        var old_used = self.occupied^
        var capacity = len(old_keys) * 2
        self.keys = List[UInt64](length=capacity, fill=0)
        self.values = List[UInt64](length=capacity, fill=0)
        self.occupied = List[UInt8](length=capacity, fill=0)
        self.used = 0
        for i in range(len(old_keys)):
            if old_used[i]:
                self.set(old_keys[i], old_values[i])

    @always_inline
    def set(mut self, key: UInt64, value: UInt64):
        if (self.used + 1) * 4 > len(self.keys) * 3:
            self.grow()
        var index = self.slot(key)
        var used = self.occupied.unsafe_ptr()
        self.used += Int(not used[index])
        used[index] = 1
        self.keys.unsafe_ptr()[index] = key
        self.values.unsafe_ptr()[index] = value

    @always_inline
    def increment(mut self, key: UInt64, delta: UInt64 = 1):
        if (self.used + 1) * 4 > len(self.keys) * 3:
            self.grow()
        var index = self.slot(key)
        var used = self.occupied.unsafe_ptr()
        var values = self.values.unsafe_ptr()
        self.used += Int(not used[index])
        used[index] = 1
        self.keys.unsafe_ptr()[index] = key
        values[index] += delta
