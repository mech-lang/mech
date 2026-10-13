Rebol [Title: "Untimed private runtime features; no benchmark admission"]
if any [not block? system/options/args 1 <> length? system/options/args] [quit/return 2]
result-file: to file! first system/options/args
print rejoin ["version=" form system/version]
print rejoin ["platform=" form system/platform]
probe-fn: func [x] [x + 1]
print rejoin ["function_is_interpreted_value=" mold function? :probe-fn]
print rejoin ["function_is_native_value=" mold native? :probe-fn]
probe-value: 1
probe-value: #{0102}
print rejoin ["dynamic_rebinding=" mold binary? probe-value]
node-a: make block! 2
node-b: make block! 2
print rejoin ["fresh_block_identity=" mold not same? node-a node-b]
before-profile: stats/profile
before-count: before-profile/recycles
print rejoin ["automatic_gc_before=" before-count]
; Allocate bounded discarded generic buffers with the runtime's default policy.
; There is no manual collector or policy-changing call in this probe.
repeat probe-index 65536 [discarded: make binary! 1024]
discarded: none
after-profile: stats/profile
print rejoin ["automatic_gc_after=" after-profile/recycles]
all-bytes: make binary! 256
repeat probe-index 256 [append all-bytes (probe-index - 1)]
write/binary result-file all-bytes
unless all-bytes = read/binary result-file [quit/return 3]
quit/return 0
