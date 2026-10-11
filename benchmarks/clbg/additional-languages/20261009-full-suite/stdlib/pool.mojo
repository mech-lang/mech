"""Owned bindings to Apache APR's existing general memory pool."""
from std.ffi import external_call, c_int
from std.memory import Pointer

def initialize():
    assert external_call["apr_initialize", c_int]() == 0

def terminate():
    external_call["apr_terminate", NoneType]()

struct Pool(Movable):
    var handle: UInt64

    def __init__(out self):
        self.handle = 0
        var status = external_call["apr_pool_create_unmanaged_ex", c_int](Pointer(to=self.handle), UInt64(0), UInt64(0))
        assert status == 0

    def allocate_bytes(self, size: Int) -> UInt64:
        assert size > 0 and self.handle != 0
        var address = external_call["apr_palloc", UInt64](self.handle, size)
        assert address != 0
        return address

    def clear(mut self):
        assert self.handle != 0
        external_call["apr_pool_clear", NoneType](self.handle)

    def destroy(mut self):
        if self.handle != 0:
            external_call["apr_pool_destroy", NoneType](self.handle)
            self.handle = 0

    def __deinit__(deinit self):
        if self.handle != 0:
            external_call["apr_pool_destroy", NoneType](self.handle)
