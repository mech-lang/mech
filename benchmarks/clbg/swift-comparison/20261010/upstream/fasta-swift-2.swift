/* The Computer Language Benchmarks Game
 https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 contributed by Ralph Ganszky
 converted to Swift 3 by Sergo Beruashvili
 software pipelining of randomFasta and optimized sequential
   performance by using UnsafeMutableBufferPointer by Dave Grove
 */

#if os(macOS)
    import Darwin
#else
    import Glibc
#endif
import Dispatch

typealias AminoAcid = (prob: Float, sym: UInt8)

let IM = UInt32(139968)
let IA = UInt32(3877)
let IC = UInt32(29573)
var seed:UInt32 = UInt32(42)

let n: Int
if CommandLine.arguments.count > 1 {
    n = Int(CommandLine.arguments[1]) ?? 1000
} else {
    n = 1000
}

let lineWidth = 60
let linesPerBlock = 1024

let aluString = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" +
    "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" +
    "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" +
    "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" +
    "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" +
    "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
    "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
var alu = aluString.utf8CString.map({ UInt8($0) })
_ = alu.popLast()

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

func write(msg: String) {
    let cStr = msg.withCString({ s in s })
    var iov = UnsafeMutablePointer<iovec>.allocate(capacity: 1)
    defer {
        iov.deallocate()
    }
    iov.pointee.iov_base = UnsafeMutableRawPointer(mutating: cStr)
    iov.pointee.iov_len = msg.utf8CString.count - 1
    writev(STDOUT_FILENO, iov, 1)
}

func write(buffer: UnsafeMutableBufferPointer<UInt8>, ofLen len: Int) {
    var iov = UnsafeMutablePointer<iovec>.allocate(capacity: 1)
    defer {
        iov.deallocate()
    }
    iov.pointee.iov_base = UnsafeMutableRawPointer(mutating: buffer.baseAddress)
    iov.pointee.iov_len = len
    writev(STDOUT_FILENO, iov, 1)
}

func repeatFasta(gene: [UInt8], n: Int) {
    let gene2 = gene + gene
    let bufSize = linesPerBlock * (lineWidth + 1)
    let raw = UnsafeMutablePointer<UInt8>.allocate(capacity: bufSize)
    defer { raw.deallocate() }
    let buffer = UnsafeMutableBufferPointer<UInt8>(start: raw, count: bufSize)
    var pos = 0
    var rpos = 0
    var cnt = n
    var lwidth = lineWidth
    while cnt > 0 {
        if pos + lwidth > buffer.count {
            write(buffer: buffer, ofLen: pos)
            pos = 0
        }
        if rpos + lwidth > gene.count {
            rpos = rpos % gene.count
        }
        if cnt < lwidth {
            lwidth = cnt
        }
        for i in 0...lwidth {
            buffer[pos+i] = gene2[rpos+i]
        }
        buffer[pos+lwidth] = 10
        pos += lwidth + 1
        rpos += lwidth
        cnt -= lwidth
    }
    if pos > 0 && pos < buffer.count {
        buffer[pos] = 10
        write(buffer: buffer, ofLen: pos)
    } else if pos == buffer.count {
        write(buffer: buffer, ofLen: pos)
        buffer[0] = 10
        write(buffer: buffer, ofLen: 1)
    }
}

func accumulateProbabilities(acid: inout [AminoAcid]) {
    for i in 1..<acid.count {
        acid[i].prob += acid[i-1].prob
    }
}

class Buffer<T> {
    let raw:UnsafeMutablePointer<T>
    let data:UnsafeMutableBufferPointer<T>
    let capacity:Int
    var filled:Int
    init(capacity:Int) {
        self.capacity = capacity;
        raw = UnsafeMutablePointer<T>.allocate(capacity: capacity)
        data = UnsafeMutableBufferPointer<T>(start: raw, count: capacity)
        filled = 0
    }
    deinit {
        raw.deallocate()
    }
    public subscript(i: Int) -> T {
        @inline(__always) get {
            return data[i]
        }

        @inline(__always) set {
            data[i] = newValue
        }
    }
}


func fillSeedBuffer(cnt: inout Int, buffer:Buffer<Float>) {
    let data = buffer.data
    let chunk = cnt > data.count ? data.count : cnt

    let f = 1.0 / Float(IM)
    var myrand : UInt32 = seed
    for i in 0..<chunk {
        myrand = (myrand * IA + IC) % IM
        buffer[i] = Float(myrand) * f
    }
    seed = myrand

    cnt -= chunk
    buffer.filled = chunk
}

func processSeedBuffer(seed:Buffer<Float>, out:Buffer<UInt8>, acid:[AminoAcid]) {
    var pos = 0
    var charCount = 0;
    let chunk = seed.filled
    let seedBuffer = seed.data
    let outputBuffer = out.data
    for i in 0..<chunk {
        let v = seedBuffer[i]
        var j = 0
        while (acid[j].prob < v) {
            j += 1
        }
        outputBuffer[pos] = acid[j].sym
        pos += 1
        charCount += 1
        if (charCount == lineWidth) {
            outputBuffer[pos] = 10
            pos += 1
            charCount = 0
        }
    }

    if (charCount > 0) {
        outputBuffer[pos] = 10
        pos += 1
    }
    write(buffer: outputBuffer, ofLen: pos)
}

func randomFasta(acid: [AminoAcid], _ n: Int) {
    var cnt = n
    let dg = DispatchGroup()
    let q = DispatchQueue.global()

    let seedBufferSize = linesPerBlock * lineWidth
    let seedBuffers = [Buffer<Float>(capacity: seedBufferSize), Buffer<Float>(capacity: seedBufferSize)]

    let outBufferSize = linesPerBlock * (lineWidth + 1)
    let outputBuffer = Buffer<UInt8>(capacity: outBufferSize)

    var color = 1 // red/black for software pipelining of loop
    fillSeedBuffer(cnt: &cnt, buffer: seedBuffers[0])
    while cnt > 0 {
        q.async(group: dg) { fillSeedBuffer(cnt: &cnt, buffer: seedBuffers[color % 2]) }
        q.async(group: dg) { processSeedBuffer(seed: seedBuffers[(color + 1) % 2], out: outputBuffer, acid: acid) }
        dg.wait()
        color += 1
    }
    processSeedBuffer(seed: seedBuffers[(color + 1) % 2], out: outputBuffer, acid: acid)
}


accumulateProbabilities(acid: &homosapiens)
accumulateProbabilities(acid: &iub)

write(msg: ">ONE Homo sapiens alu\n")
repeatFasta(gene: alu, n: 2*n)

write(msg: ">TWO IUB ambiguity codes\n")
randomFasta(acid: iub,3*n)

write(msg: ">THREE Homo sapiens frequency\n")
randomFasta(acid: homosapiens,5*n)
