"""Physical-only adapter to the exact qualified Nim lexical mask.

No owner, normalized LOC, compiler, formatter or application entry is called.
The guarded worker pins this shim and the original Nim lexical dependencies.
"""
from foreign_nim import mask_nim


def mask_foreign(source, language):
    if language != 'nim':
        raise ValueError('This reviewed physical mask accepts only Nim')
    return mask_nim(source)
