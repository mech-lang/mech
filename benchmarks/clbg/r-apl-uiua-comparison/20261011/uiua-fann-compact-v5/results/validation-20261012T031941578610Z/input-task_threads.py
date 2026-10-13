"""Read-only native macOS task thread observations; no workload computation."""
import ctypes


class TaskInfo(ctypes.Structure):
    _fields_ = [(name, ctypes.c_uint64) for name in
                ('virtual_size', 'resident_size', 'total_user', 'total_system', 'threads_user', 'threads_system')]
    _fields_ += [(name, ctypes.c_int32) for name in
                 ('policy', 'faults', 'pageins', 'cow_faults', 'messages_sent', 'messages_received',
                  'syscalls_mach', 'syscalls_unix', 'csw', 'threadnum', 'numrunning', 'priority')]


class TaskThreads:
    def __init__(self):
        self.library = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
        self.library.proc_pidinfo.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_uint64,
                                             ctypes.c_void_p, ctypes.c_int]
        self.library.proc_pidinfo.restype = ctypes.c_int

    def read(self, pid):
        value = TaskInfo()
        actual = self.library.proc_pidinfo(pid, 4, 0, ctypes.byref(value), ctypes.sizeof(value))
        if actual != ctypes.sizeof(value):
            return None
        return dict(pid=pid, total_os_threads=value.threadnum, running_os_threads=value.numrunning,
                    resident_bytes=value.resident_size)
