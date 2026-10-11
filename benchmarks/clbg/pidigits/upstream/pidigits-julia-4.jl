# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

# contributed by Brad Chamberlain
# based on the Chapel #2 version and Python (#1) version

using Printf

function pidigits(numDigits::Int)
    numer,accum,denom,tmp1,tmp2 = map(BigInt,(1,0,1,0,0))

    i = k = 1
    ns = d = d2 = 0

    while i <= numDigits
        (accum,numer,denom) = nextTerm(k,accum,numer,denom)
        k += 1
        if numer <= accum
            (d,tmp1,tmp2) = extractDigit(3,numer,accum,denom,tmp1,tmp2)
            (d2,tmp1,tmp2) = extractDigit(4,numer,accum,denom,tmp1,tmp2)
            if d == d2
                ns = ns * 10 + d
                if i % 10 == 0
                    @printf("%010d\t:%d\n",ns,i)
                    ns = 0
                end
 
                (accum,numer) = eliminateDigit(d,accum,denom,numer)
                i += 1
            end
        end
    end

    # Pad out any trailing digits for the final line
    if numDigits % 10 != 0
        @printf("%s",lpad(ns,numDigits%10,"0"))
        for i in 1:(10-numDigits%10)
            @printf(" ")
        end
        @printf("\t:%d\n",n)
     end
end

function nextTerm(k::Int64, accum::BigInt, numer::BigInt, denom::BigInt)
    k2 = 2 * k + 1

    accum += numer*2
    accum *= k2
    denom *= k2
    numer *= k
    (accum,numer,denom)
end

function extractDigit(nth::Int64, numer::BigInt, accum::BigInt, denom::BigInt, tmp1::BigInt, tmp2::BigInt)
    tmp1 = numer * nth
    tmp2 = tmp1 + accum
    (tmp1,_) = divrem(tmp2,denom)

    (convert(Int64,tmp1),tmp1,tmp2)
end

function eliminateDigit(d::Int64, accum::BigInt, denom::BigInt, numer::BigInt)
    accum -= denom * d
    accum *= 10
    numer *= 10
    (accum,numer)
end

n = parse(Int,ARGS[1])
pidigits(n)
