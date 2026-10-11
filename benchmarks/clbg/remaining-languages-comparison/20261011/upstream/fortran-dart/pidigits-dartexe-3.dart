/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Denis Gribov
   based on C source code by Ledhug
   un-buffered and transliterated to Dart by Isaac Gouy
   
*/

void main(List<String> arguments) {
  final n = int.parse(arguments[0]);

  final bigIntThree = new BigInt.from(3);
  final bigIntTen = new BigInt.from(10);

  var i = 0, ns = 0;
  var k = 0;
  var k2 = 1;
  var acc = BigInt.zero;
  var den = BigInt.one;
  var num = BigInt.one;
  var tmp = BigInt.zero;
  var d3 = BigInt.zero;
  var d4 = BigInt.zero;

  while (i < n) {
    k++;
    k2 += 2;

    acc += num * BigInt.two;
    acc *= new BigInt.from(k2);
    den *= new BigInt.from(k2);
    num *= new BigInt.from(k);

    if (num > acc) {
      continue;
    }

    tmp = num * bigIntThree;
    tmp = tmp + acc;
    d3 = tmp ~/ den;

    tmp = tmp + num;
    d4 = tmp ~/ den;

    if (d3 != d4) {
      continue;
    }

    final d = d3.toInt();
    ns = ns * 10 + d;
    i++;
    var last = i >= n;
    if (i % 10 == 0 || last) {
      print(pad(ns, last) + '\t:$i');
      ns = 0;
    }

    if (last) break;

    acc -= den * new BigInt.from(d);
    acc *= bigIntTen;
    num *= bigIntTen;
  }
}

String pad(i, last) {
  var res = i.toString(), count;
  count = 10 - res.length;
  while (count > 0) {
    last ? res += ' ' : res = '0' + res;
    count--;
  }
  return res;
}
