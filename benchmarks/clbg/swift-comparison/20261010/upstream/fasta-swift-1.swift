/* The Computer Language Benchmarks Game
 https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 contributed by Ralph Ganszky
 converted to Swift 3 by Sergo Beruashvili
 */

import Glibc
//import Darwin
typealias AminoAcid = (prob: Double, sym: UInt8)

let IM = 139968
let IA = 3877
let IC = 29573
var seed = 42

let n: Int
if CommandLine.arguments.count > 1 {
    n = Int(CommandLine.arguments[1]) ?? 1000
} else {
    n = 1000
}

let bufferSize = 256*1024
let width = 60

let aluString = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" +
    "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" +
    "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" +
    "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" +
    "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" +
    "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
"AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
var alu = aluString.utf8CString.map({ UInt8($0) })
alu.popLast()

var iub = [
    AminoAcid(0.27, 97), // "a"),
    AminoAcid(0.12, 99), // "c"),
    AminoAcid(0.12, 103), // "g"),
    AminoAcid(0.27, 116), // "t"),
    AminoAcid(0.02, 66), // "B"),
    AminoAcid(0.02, 68), // "D"),
    AminoAcid(0.02, 72), // "H"),
    AminoAcid(0.02, 75), // "K"),
    AminoAcid(0.02, 77), // "M"),
    AminoAcid(0.02, 78), // "N"),
    AminoAcid(0.02, 82), // "R"),
    AminoAcid(0.02, 83), // "S"),
    AminoAcid(0.02, 86), // "V"),
    AminoAcid(0.02, 87), // "W"),
    AminoAcid(0.02, 89), // "Y"),
]

var homosapiens = [
    AminoAcid(0.3029549426680, 97), // "a"),
    AminoAcid(0.1979883004921, 99), // "c"),
    AminoAcid(0.1975473066391, 103), // "g"),
    AminoAcid(0.3015094502008, 116), // "t"),
]

func write(array: [UInt8], ofLen len: Int) {
    var iov = UnsafeMutablePointer<iovec>.allocate(capacity: 1)
    defer {
        iov.deallocate()
    }
    let data = array.withUnsafeBufferPointer({ p in p })
    iov.pointee.iov_base = UnsafeMutableRawPointer(mutating: data.baseAddress)
    iov.pointee.iov_len = len
    writev(STDOUT_FILENO, iov, 1)
}

func write(buffer: UnsafePointer<Int8>, ofLen len: Int) {
    var iov = UnsafeMutablePointer<iovec>.allocate(capacity: 1)
    defer {
        iov.deallocate()
    }
    iov.pointee.iov_base = UnsafeMutableRawPointer(mutating: buffer)
    iov.pointee.iov_len = len
    writev(STDOUT_FILENO, iov, 1)
}

func repeatFasta(gene: [UInt8], n: Int) {
    let gene2 = gene + gene
    var buffer = [UInt8](repeating: 10, count: bufferSize)
    var pos = 0
    var rpos = 0
    var cnt = n
    var lwidth = width
    while cnt > 0 {
        if pos + lwidth > buffer.count {
            write(array: buffer, ofLen: pos)
            pos = 0
        }
        if rpos + lwidth > gene.count {
            rpos = rpos % gene.count
        }
        if cnt < lwidth {
            lwidth = cnt
        }
        buffer[pos..<pos+lwidth] = gene2[rpos..<rpos+lwidth]
        buffer[pos+lwidth] = 10
        pos += lwidth + 1
        rpos += lwidth
        cnt -= lwidth
    }
    if pos > 0 && pos < buffer.count {
        buffer[pos] = 10
        write(array: buffer, ofLen: pos)
    } else if pos == buffer.count {
        write(array: buffer, ofLen: pos)
        buffer[0] = 10
        write(array: buffer, ofLen: 1)
    }
}

func search(rnd: Double, within arr: [AminoAcid]) -> UInt8 {
    var low = 0
    var high = arr.count - 1
    while low <= high {
        let mid = low + (high - low) / 2
        if arr[mid].prob >= rnd {
            high = mid - 1
        } else {
            low = mid + 1
        }
    }
    return arr[high+1].sym
}

func accumulateProbabilities( acid: inout [AminoAcid]) {
    for i in 1..<acid.count {
        acid[i].prob += acid[i-1].prob
    }
}

func randomFasta( acid: inout [AminoAcid], _ n: Int) {
    var cnt = n
    accumulateProbabilities(acid: &acid)
    var buffer = [UInt8](repeating: 10, count: bufferSize)
    var pos = 0
    while cnt > 0 {
        var m = cnt
        if m > width {
            m = width
        }
        let f = 1.0 / Double(IM)
        var myrand = seed
        for _ in 0..<m {
            myrand = (myrand * IA + IC) % IM
            let r = Double(myrand) * f
            buffer[pos] = search(rnd: r, within: acid)
            pos += 1
            if pos == buffer.count {
                write(array: buffer, ofLen: pos)
                pos = 0
            }
        }
        seed = myrand
        buffer[pos] = 10
        pos += 1
        if pos == buffer.count {
            write(array: buffer, ofLen: pos)
            pos = 0
        }
        cnt -= m
    }
    if pos > 0 {
        write(array: buffer, ofLen: pos)
    }
}

let one = ">ONE Homo sapiens alu\n"
let oneStr = one.withCString({ s in s })
write(buffer: oneStr, ofLen: one.utf8CString.count - 1)
repeatFasta(gene: alu, n: 2*n)

let two = ">TWO IUB ambiguity codes\n"
let twoStr = two.withCString({ s in s })
write(buffer: twoStr, ofLen: two.utf8CString.count - 1)
randomFasta(acid: &iub,3*n)

let three = ">THREE Homo sapiens frequency\n"
let threeStr = three.withCString({ s in s })
write(buffer: threeStr, ofLen: three.utf8CString.count - 1)
randomFasta(acid: &homosapiens,5*n)
