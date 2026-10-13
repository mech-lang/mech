# Additive C-family literal mask provenance

This working change retains exact source-offset evidence for string token
regions already masked by the checked Pygments pass. Adjacent string-token
fragments merge only when their exact offsets touch; whitespace-separated
literal tokens remain separate. Include filenames keep their separate import
owner. Source text, mask bytes, newlines, physical/normalized accounting and
the frozen job18 are unchanged.

The new `masked_literal_spans` field proves only which original characters
became the existing literal mask. It does not certify prefixes, concatenation,
expression purity, declaration roles or a complete literal grammar. An actual
identifier `S`/`SSS` has no mask provenance. A future C++ direct-declarator
consumer may conservatively gate any envelope overlapping these regions;
it must not interpret an unowned mask as an unknown bare-name invariant.

Eight named fixture methods cover string-token fragments, literal separation,
raw string rows, include ownership, comments, real mask-like identifiers,
character literals and encoding prefixes. They have not executed. No lexer,
normalizer, source/hash sweep, build, timing or chart admission ran for this
change. Qualification belongs in a future immutable common-mutex job after
job18 terminates.
