package clbg.binarytrees
import clbg.Support

class Node(val left: Node?, val right: Node?)
fun tree(depth: Int): Node = if (depth == 0) Node(null, null) else Node(tree(depth - 1), tree(depth - 1))
fun check(node: Node): Long = if (node.left == null) 1L else 1L + check(node.left) + check(node.right!!)
fun main(args: Array<String>) {
    val maximum = maxOf(6, Support.size(args, 10))
    val out = StringBuilder()
    out.append("stretch tree of depth ${maximum + 1}\t check: ${check(tree(maximum + 1))}\n")
    val longLived = tree(maximum)
    var depth = 4
    while (depth <= maximum) {
        val iterations = 1L shl (maximum - depth + 4)
        var total = 0L
        var i = 0L
        while (i < iterations) { total += check(tree(depth)); i++ }
        out.append("$iterations\t trees of depth $depth\t check: $total\n")
        depth += 2
    }
    out.append("long lived tree of depth $maximum\t check: ${check(longLived)}\n")
    Support.write(out.toString())
}
