// The Computer Language Benchmarks Game
// http://benchmarksgame.alioth.debian.org/
//
// based on programs by Marcel Ibes and Ralph Ganszky
// contributed by Isaac Gouy


import Dispatch
import Foundation

class TreeNode {
    var left, right: TreeNode?

    init(left: TreeNode?, right: TreeNode?) {
        self.left = left
	self.right = right
    }

    func check() -> Int {
	if left != nil {
	    return left!.check() + right!.check() + 1
	} else {
	    return 1
	}
    }
}

func bottomUpTree(_ depth: Int) -> TreeNode? {
    if depth > 0 {
	let node = TreeNode(left: bottomUpTree(depth-1),
			    right: bottomUpTree(depth-1))
	return node
    } else {
	let node = TreeNode(left: nil, right: nil)
	return node
    }
}

func inner(depth: Int, iterations: Int) -> String {
    var chk = 0
    for _ in 0..<iterations {
        let a = bottomUpTree(depth)
        chk += a!.check()
    }
    return "\(iterations)\t trees of depth \(depth)\t check: \(chk)"
}

let n: Int

if CommandLine.argc > 1 {
    n = Int(CommandLine.arguments[1]) ?? 10
} else {
    n = 10
}

let minDepth = 4
let maxDepth = (n > minDepth + 2) ? n : minDepth + 2
var messages: [Int:String] = [:]
let depth = maxDepth+1

let group = DispatchGroup()

let workerQueue = DispatchQueue.init(label: "workerQueue", qos: .userInitiated, attributes: .concurrent)
let messageQueue = DispatchQueue.init(label: "messageQueue", qos: .userInitiated)

group.enter()
workerQueue.async {
    let tree = bottomUpTree(depth)
    
    messageQueue.async {
        messages[0] = "stretch tree of depth \(depth)\t check: \(tree!.check())"
        group.leave()
    }
}

group.enter()
workerQueue.async {
    let longLivedTree = bottomUpTree(maxDepth)
    
    messageQueue.async {
        messages[Int.max] = "long lived tree of depth \(maxDepth)\t check: \(longLivedTree!.check())"
        group.leave()
    }
}

group.enter()
workerQueue.async {
    for halfDepth in (minDepth / 2)..<(maxDepth/2+1) {
        let depth = halfDepth * 2
        let iterations = 1 << (maxDepth - depth + minDepth)

        group.enter()
        workerQueue.async {
            let msg = inner(depth: depth, iterations: iterations)
            messageQueue.async {
                messages[depth] = msg
                group.leave()
            }
        }
    }
    
    messageQueue.async {
        group.leave()
    }
}

group.wait()

for msg in messages.sorted(by: { $0.0 < $1.0 }) {
    print(msg.value)
}
