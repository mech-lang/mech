/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Ralph Ganszky
   modified by Michael Morrell

   No match for key (for example "GGTATTTTAATT") causes --
   "fatal error: unexpectedly found nil while unwrapping an Optional value"

*/

import Glibc
import Dispatch

let ENABLED_THREAD_COUNT = 4
let ntasks = ENABLED_THREAD_COUNT

let mQueue = DispatchQueue(label: "mergeQueue")

func compress(_ seq: ArraySlice<UInt8>) -> Int {
    var res = 0
    for i in seq.indices {
        res = (res << 2) | Int(seq[i])
    }
    return res
}

func getSequenceHash(_ n: Int, seq: [UInt8]) -> [Int:Int] {
    var hash = [Int:Int]()
    let slice = (seq.count-(n-1)) / ntasks
    let remainder = (seq.count-(n-1)) % ntasks
    let mask = n > 1 ? ((1 << (2*(n-1))) - 1) : 0
    DispatchQueue.concurrentPerform(iterations: ntasks) { i in
        var lHash = [Int:Int](minimumCapacity: 1 << min(n, 12))
        var idx = compress(seq[i*slice..<i*slice+n])
        lHash[idx] = (lHash[idx] ?? 0) + 1
        let startIdx = i*slice+n
        let endIdx = startIdx + slice - 1
        for l in startIdx..<endIdx {
            idx = ((idx & mask) << 2) | Int(seq[l])
            lHash[idx] = (lHash[idx] ?? 0) + 1
        }
        mQueue.sync {
            for (key, value) in lHash {
                hash[key] = (hash[key] ?? 0) + value
            }
        }
    }
    let startIdx = seq.count - remainder - (n - 1)
    let endIdx = seq.count - (n - 1)
    for i in startIdx..<endIdx {
        let idx = compress(seq[i..<i+n])
        hash[idx] = (hash[idx] ?? 0) + 1
    }
    return hash
}

let c2i: [Character:Int] = [ "A": 0, "C": 1, "T": 2, "G": 3 ]

func encode(_ seq: String) -> Int {
    let cSeq = seq.characters
    var res = 0
    for c in cSeq {
        res = res << 2 | c2i[c]!
    }
    return res
}

func roundDouble(_ num: Double, precision: Int) -> String {
    let exponent = pow(10.0, Double(precision))
    let number = Double(Int(num * exponent + 0.5)) / exponent
    var numberStr = "\(number)"
    while numberStr.characters.count < Int(log10(num)) + 2 + precision {
        numberStr = numberStr + "0"
    }
    return numberStr
}

func readInput() -> [UInt8] {
    var seq = [UInt8]()
    let pattern = ">THREE Homo sapiens frequency"

    while let line = readLine() {
        if line == pattern {
            break
        }
    }

    while let line = readLine() {
        seq += Array(line.utf8)
    }

    return seq
}

// Read sequence
var sequence = readInput()

// rewrite bytes with 2bit code
for i in 0..<sequence.count {
    sequence[i] = (sequence[i] & 0x6) >> 1
}

let hash = getSequenceHash(1, seq: sequence)

let i2c = [ 0: "A", 1: "C", 2: "T", 3: "G" ]

let total = hash.reduce(0) { $0 + $1.1 }
for k in hash.keys.sorted(by: {hash[$1]! < hash[$0]!}) {
    print("\(i2c[k]!) \(roundDouble(100.0*Double(hash[k]!)/Double(total), precision: 3))")
}
print()

let hash2 = getSequenceHash(2, seq: sequence)

let total2 = hash2.reduce(0) { $0 + $1.1 }
for k in hash2.keys.sorted(by: {hash2[$1]! < hash2[$0]!}) {
    print("\(i2c[k>>2]!)\(i2c[k&3]!) \(roundDouble(100.0*Double(hash2[k]!)/Double(total2), precision: 3))")
}
print()

let hash3 = getSequenceHash(3, seq: sequence)
print("\(hash3[encode("GGT")]!)\tGGT")

let hash4 = getSequenceHash(4, seq: sequence)
print("\(hash4[encode("GGTA")]!)\tGGTA")

let hash6 = getSequenceHash(6, seq: sequence)
print("\(hash6[encode("GGTATT")]!)\tGGTATT")

let hash12 = getSequenceHash(12, seq: sequence)
print("\(hash12[encode("GGTATTTTAATT")]!)\tGGTATTTTAATT")

let hash18 = getSequenceHash(18, seq: sequence)
print("\(hash18[encode("GGTATTTTAATTTATAGT")]!)\tGGTATTTTAATTTATAGT")
