"""Select Linux's multiprocessing policy, then execute an unchanged source file.

The pinned parallel CLBG sources assume inherited file descriptors and globals.
This launcher changes only the runtime start policy on macOS; it is timed.
"""
import multiprocessing
import runpy
import sys

multiprocessing.set_start_method('fork')
sys.argv = sys.argv[1:]
runpy.run_path(sys.argv[0], run_name='__main__')
