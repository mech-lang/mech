# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Use libgmp-ruby_1.0 
#
# contributed by Gabriele Renzi
# modified by Pilho Kim
# fixed by SO Alex

require_relative "gmp.so"

class PiDigitSpigot
    def initialize()
        @ZERO = GMP::Z.new(0)
        @ONE = GMP::Z.new(1)
        @THREE = GMP::Z.new(3)
        @FOUR = GMP::Z.new(4)
        @TEN = GMP::Z.new(10)
        @z = Transformation.new @ONE,@ZERO,@ZERO,@ONE
        @x = Transformation.new @ZERO,@ZERO,@ZERO,@ZERO
        @inverse = Transformation.new @ZERO,@ZERO,@ZERO,@ZERO
    end

    def next!
        @y = @z.extract(@THREE)
        if safe? @y
            @z = produce(@y)
            @y
        else
            @z = consume @x.next!()
            next!()
        end
    end

    def safe?(digit)
        digit == @z.extract(@FOUR)
    end

    def produce(i)
        @inverse.qrst(@TEN,-@TEN*i,@ZERO,@ONE).compose(@z)
    end

    def consume(a)
        @z.compose(a)
    end
end


class Transformation
    attr_reader :q, :r, :s, :t
    def initialize (q, r, s, t)
        @ZERO = GMP::Z.new(0)
        @ONE = GMP::Z.new(1)
        @TWO = GMP::Z.new(2)
        @FOUR = GMP::Z.new(4)
        @q,@r,@s,@t,@k = q,r,s,t,@ZERO
    end

    def next!()
        @q = @k = @k + @ONE
        @r = @FOUR * @k + @TWO
        @s = @ZERO
        @t = @TWO * @k + @ONE
        self
    end

    def extract(j)
        (@q * j + @r).tdiv( @s * j + @t )
    end

    def compose(a)
        self.class.new( @q * a.q,
                        @q * a.r + r * a.t,
                        @s * a.q + t * a.s,
                        @s * a.r + t * a.t
                    )
    end

    def qrst *args
        initialize *args
        self
    end

end


@zero = GMP::Z.new(0)
@one = GMP::Z.new(1)
@two = GMP::Z.new(2)
@four = GMP::Z.new(4)
@ten = GMP::Z.new(10)

WIDTH = 10
n = Integer(ARGV[0] || "27")
j = 0

digits = PiDigitSpigot.new

while n > 0
    if n >= WIDTH
        WIDTH.times {print digits.next!}
        j += WIDTH
    else
        n.times {print digits.next!}
        (WIDTH-n).times {print " "}
        j += n
    end
    puts "\t:"+j.to_s
    n -= WIDTH
end
