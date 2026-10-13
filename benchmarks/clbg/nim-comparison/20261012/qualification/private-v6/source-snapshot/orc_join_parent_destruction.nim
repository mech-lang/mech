import std/[tables, typedthreads, sets, atomics, os]
type Job = tuple[count: int, slot: int, output: ptr Table[uint64, string], tid: ptr int, ready: ptr Atomic[int], go: ptr Atomic[bool]]
proc worker(job: Job) {.thread.} =
  job.output[] = initTable[uint64, string]()
  job.tid[] = getThreadId()
  discard job.ready[].fetchAdd(1, moRelease)
  while not job.go[].load(moAcquire):
    sleep(1)
  for i in 0 ..< job.count:
    let key = uint64(i * 17 + job.slot)
    job.output[][key] = $key & ":" & $job.slot
for count in [0, 1, 2, 17, 65, 1025, 4096]:
  var tables: array[7, Table[uint64, string]]
  var tids: array[7, int]
  var threads: array[7, Thread[Job]]
  var ready: Atomic[int]
  var go: Atomic[bool]
  for slot in 0 ..< 7:
    createThread(threads[slot], worker, (count, slot, addr tables[slot], addr tids[slot], addr ready, addr go))
  while ready.load(moAcquire) != 7:
    sleep(1)
  go.store(true, moRelease)
  for slot in 0 ..< 7:
    joinThread(threads[slot])
  var threadIds = initHashSet[int]()
  for slot in 0 ..< 7:
    threadIds.incl(tids[slot])
    doAssert tables[slot].len == count
    for i in 0 ..< count:
      let key = uint64(i * 17 + slot)
      doAssert tables[slot][key] == $key & ":" & $slot
  doAssert threadIds.len == 7
  reset(tables)
  for slot in 0 ..< 7:
    doAssert tables[slot].len == 0
echo "orc-seven-worker-join-parent-destruction-ok"
