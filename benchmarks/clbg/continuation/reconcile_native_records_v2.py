"""Add Gleam and Clojure to the unchanged, fixture-qualified record contract.

The v1 file and its passed fixtures remain immutable. A private function copy
changes only the language inventory; it never rewrites source identities,
commands, native records or timings. JVM/BEAM processes follow the same actual
nine-record ownership/output/environment protocol. This module is a candidate
until its additive fixtures pass under the common timing mutex.
"""
from types import FunctionType

import reconcile_native_records_v1 as original

LANGUAGES = {'nim', 'crystal', 'pony', 'c3', 'gleam', 'clojure'}
require = original.require
exact_integer = original.exact_integer

_globals = dict(original.validate_benchmark.__globals__, LANGUAGES=LANGUAGES)
validate_benchmark = FunctionType(original.validate_benchmark.__code__, _globals,
    name='validate_benchmark', argdefs=original.validate_benchmark.__defaults__)
validate_benchmark.__kwdefaults__ = dict(original.validate_benchmark.__kwdefaults__ or {})

