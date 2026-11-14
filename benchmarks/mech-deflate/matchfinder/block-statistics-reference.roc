import CompressOptimal
import CompressLazy

set32 = |xs, at, value| match List.set(xs, at, value) {
    Ok(next) => next
    Err(_) => crash "oracle counter write"
}

expect {
    total = set32(set32(List.repeat(0.U32, 259), 3, 5), 258, 100)
    fresh = set32(set32(List.repeat(0.U32, 259), 3, 7), 258, 1)
    expected = set32(set32(List.repeat(0.U32, 259), 3, 12), 258, 101)
    actual = CompressOptimal.merge_match_len_freqs(total, fresh)
    actual.total == expected and actual.fresh == List.repeat(0.U32, 259)
}

expect {
    fresh : List(U32)
    fresh = [77, 77, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
    total : List(U32)
    total = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    actual = CompressLazy.merge_observations(fresh, 2, total, 512, 1024)
    actual.new_observations == [77, 77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        and actual.observations == [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        and actual.num_new_observations == 0
        and actual.num_observations == 1536
}
