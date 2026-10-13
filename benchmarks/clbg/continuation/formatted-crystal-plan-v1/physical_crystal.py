"""Physical-only adapter to the exact already qualified Crystal lexical mask.

The guarded worker pins this adapter and all imported modules. It invokes no
scope classifier, semantic normalization, producer count or application.
"""
from foreign_crystal import mask_crystal


def mask_foreign(source, language):
    if language != 'crystal':
        raise ValueError('This reviewed physical mask accepts only Crystal')
    return mask_crystal(source)
