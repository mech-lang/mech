#!/usr/bin/env python3
"""Check top-five optimality independently using tag-subset moments.

The benchmark datasets contain four/five distinct tags per post. Enumerating
all subsets is practical for that shape, and supports arbitrary UTF-8 tags.
This verifier has exponential cost in tags per post; it is not a general
100-tags-per-post algorithm. Exact Rust fixture mode deliberately preserves
upstream's zero-score fallback (repeated post index zero) on sparse inputs.
"""

import argparse
import collections
import itertools
import json
import math
from pathlib import Path


def expected_scores(posts):
    subset_counts = collections.Counter()
    canonical = []
    for post in posts:
        tags = tuple(sorted(post["tags"]))
        assert len(tags) == len(set(tags)), "this verifier expects distinct tags per post"
        canonical.append(tags)
        for size in range(len(tags) + 1):
            subset_counts.update(itertools.combinations(tags, size))
    by_shape = {}
    for tags in set(canonical):
        size = len(tags)
        # Sum C(shared_tags,k) over all other posts, then invert these moments.
        moments = [sum(subset_counts[subset] for subset in itertools.combinations(tags, k)) - math.comb(size, k) for k in range(size + 1)]
        histogram = [0] * (size + 1)
        for k in range(size, -1, -1):
            histogram[k] = moments[k] - sum(math.comb(j, k) * histogram[j] for j in range(k + 1, size + 1))
        assert sum(histogram) == len(posts) - 1 and min(histogram) >= 0
        left, score = min(5, len(posts) - 1), 0
        for k in range(size, -1, -1):
            take = min(left, histogram[k])
            left -= take
            score += take * k
        assert left == 0
        by_shape[tags] = score
    return {post["_id"]: by_shape[tags] for post, tags in zip(posts, canonical)}


def validate(posts, actual, scores, expected_indices=None):
    assert len(actual) == len(posts), (len(actual), len(posts))
    by_id = {post["_id"]: post for post in posts}
    assert len(by_id) == len(posts)
    if expected_indices is not None:
        assert len(expected_indices) == len(posts)
    for index, (post, row) in enumerate(zip(posts, actual)):
        assert set(row) == {"_id", "tags", "related"}
        assert row["_id"] == post["_id"] and row["tags"] == post["tags"]
        related = row["related"]
        assert len(related) == 5
        assert all(item == by_id[item["_id"]] for item in related), row["_id"]
        if expected_indices is not None:
            assert related == [posts[i] for i in expected_indices[index]], row["_id"]
        else:
            ids = [item["_id"] for item in related]
            assert post["_id"] not in ids and len(set(ids)) == 5, row["_id"]
        actual_score = sum(len(set(post["tags"]) & set(item["tags"])) for item in related)
        expected_score = scores[row["_id"]] if expected_indices is None else sum(len(set(post["tags"]) & set(posts[i]["tags"])) for i in expected_indices[index])
        assert actual_score == expected_score, (row["_id"], actual_score, expected_score)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("actual", type=Path, nargs="?")
    parser.add_argument("--reference", type=Path, help="Read or save independently calculated score totals")
    parser.add_argument("--expected-indices", type=Path, help="Fixture expected related_indices; permits exact Rust sparse-input behavior")
    args = parser.parse_args()
    posts = json.loads(args.source.read_text())
    scores = expected_scores(posts)
    reference = args.reference or args.source.with_name("correct-" + args.source.name.removeprefix("posts-"))
    if reference.exists():
        assert json.loads(reference.read_text()) == scores, "reference scores changed"
    else:
        reference.write_text(json.dumps(scores, sort_keys=True) + "\n")
    expected = None
    if args.expected_indices:
        expected = json.loads(args.expected_indices.read_text())["related_indices"]
    if args.actual:
        validate(posts, json.loads(args.actual.read_text()), scores, expected)
    print(json.dumps({"posts": len(posts), "unique_tag_sets": len({tuple(post["tags"]) for post in posts}), "status": "passed", "reference": str(reference)}))


if __name__ == "__main__":
    main()
