/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Isaac Gouy, updated Alexander Fyodorov's Dart #2
*/

String pad(i, last) {
  var res = i.toString(), count;
  count = 10 - res.length;
  while (count > 0) {
    last ? res += ' ' : res = '0' + res;
    count--;
  }
  return res;
}

void calculatePi(arg) {
  var i = 0, ns = 0;

  final bigint_three = new BigInt.from(3);
  final bigint_ten = new BigInt.from(10);

  var k = BigInt.zero;
  var k1 = BigInt.one;
  var a = BigInt.zero;
  var d = BigInt.one;
  var m = BigInt.zero;
  var n = BigInt.one;
  var t = BigInt.zero;
  var u = BigInt.one;

  while (true) {
    k += BigInt.one;
    k1 += BigInt.two;
    t = n << 1;
    n *= k;
    a += t;
    a *= k1;
    d *= k1;

    if (a.compareTo(n) >= 0) {
      m = n * bigint_three + a;
      t = m ~/ d;
      u = m % d + n;

      if (d.compareTo(u) > 0) {
        ns = ns * 10 + t.toInt();
        i += 1;

        var last = i >= arg;
        if (i % 10 == 0 || last) {
          print(pad(ns, last) + '\t:$i');
          ns = 0;
        }

        if (last) break;

        a = (a - d * t) * bigint_ten;
        n = n * bigint_ten;
      }
    }
  }
}

void main(List<String> arguments) {
  calculatePi( int.parse(arguments[0]) );
}
