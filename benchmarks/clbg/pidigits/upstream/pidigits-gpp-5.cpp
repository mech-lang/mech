/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Basit Ayantunde
 */

#include <cinttypes>
#include <iostream>

#include "gmp.h"

struct Mpz {
  Mpz() noexcept { __gmpz_init_set_ui(&mpz_, 0); }
  ~Mpz() noexcept { __gmpz_clear(&mpz_); }

  explicit Mpz(ulong value) noexcept { __gmpz_init_set_ui(&mpz_, value); }

  void set_value(ulong value) noexcept { __gmpz_init_set_ui(&mpz_, value); }

  // *this = a * b
  void mul(Mpz const& a, ulong b) noexcept { __gmpz_mul_ui(&mpz_, &a.mpz_, b); }

  // *this *= a
  void mul(ulong b) noexcept { __gmpz_mul_ui(&mpz_, &mpz_, b); }

  // *this -= a * b
  void submul(Mpz const& a, ulong b) noexcept {
    __gmpz_submul_ui(&mpz_, &a.mpz_, b);
  }

  // *this = a + b
  void add(Mpz const& a, Mpz const& b) noexcept {
    __gmpz_add(&mpz_, &a.mpz_, &b.mpz_);
  }

  // *this += a
  void add(Mpz const& a) noexcept { __gmpz_add(&mpz_, &mpz_, &a.mpz_); }

  // *this += a
  void add(ulong a) noexcept { __gmpz_add_ui(&mpz_, &mpz_, a); }

  // *this += a * b
  void addmul(Mpz const& a, ulong b) noexcept {
    __gmpz_addmul_ui(&mpz_, &a.mpz_, b);
  }

  // *this = a / b
  void tdiv_q(Mpz const& a, Mpz const& b) noexcept {
    __gmpz_tdiv_q(&mpz_, &a.mpz_, &b.mpz_);
  }

  ulong as_ui() const noexcept { return __gmpz_get_ui(&mpz_); }

  bool operator>(Mpz const& other) const noexcept {
    return mpz_cmp(&mpz_, &other.mpz_) > 0;
  }

 private:
  __mpz_struct mpz_;
};

struct Context {
  Mpz tmp1, tmp2;
};

class LFT {
 public:
  Mpz q;
  Mpz r;
  Mpz t;
  ulong k;

 public:
  LFT() noexcept : q(1), r(0), t(1), k(0) {}

  void next() noexcept {
    ++k;
    r.addmul(q, 2);
    r.mul(2 * k + 1);
    t.mul(2 * k + 1);
    q.mul(k);
  }

  ulong extract(Context& ctx, ulong x) const noexcept {  // NOLINT
    ctx.tmp1.mul(q, x);
    ctx.tmp1.add(r);
    ctx.tmp2.tdiv_q(ctx.tmp1, t);
    return ctx.tmp2.as_ui();
  }

  void produce(ulong n) noexcept {
    q.mul(10);
    r.submul(t, n);
    r.mul(10);
  }
};

int main(int, char** argv) {
  std::ios_base::sync_with_stdio(false);
  size_t const total_digits = std::atol(argv[1]);

  auto context = Context();
  LFT lft;
  size_t n_digits = 0;
  while (n_digits < total_digits) {
    size_t i = 0;
    while (i < 10 && n_digits < total_digits) {
      lft.next();
      if (lft.q > lft.r) continue;

      auto digit = lft.extract(context, 3);
      if (digit == lft.extract(context, 4)) {
        std::cout << digit;
        lft.produce(digit);
        ++i;
        ++n_digits;
      }
    }

    // Pad digits with extra spaces if total_digits was not a
    // multiple of 10.
    for (; i < 10; ++i) std::cout << ' ';
    std::cout << "\t:" << n_digits << '\n';
  }
}
