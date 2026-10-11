// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Contributed by Kevin Miller ( as C code )
//
// Ported to C++ with minor changes by Dave Compton
// Optimized to x86 by Kenta Yoshimura
//
// Compile with following g++ flags
// Use '-O3 -ffp-contract=off -fno-expensive-optimizations' instead of '-Ofast',
// because FMA is fast, but different precision to original version
//   -Wall -O3 -ffp-contract=off -fno-expensive-optimizations -march=native -fopenmp --std=c++14 mandelbrot.cpp

#include <immintrin.h>
#include <stdio.h>
#include <stdint.h>

using namespace std;

namespace {

#if defined(__AVX512BW__)
    typedef __m512d Vec;
    Vec vec_init(double value)       { return _mm512_set1_pd(value); }
    bool vec_is_any_le(Vec v, Vec f) { return bool(_mm512_cmp_pd_mask(v, f, _CMP_LE_OS)); }
    int vec_is_le(Vec v1, Vec v2)    { return _mm512_cmp_pd_mask(v1, v2, _CMP_LE_OS); }
    const uint8_t k_bit_rev[] =
    {
        0x00, 0x80, 0x40, 0xC0, 0x20, 0xA0, 0x60, 0xE0, 0x10, 0x90, 0x50, 0xD0, 0x30, 0xB0, 0x70, 0xF0,
        0x08, 0x88, 0x48, 0xC8, 0x28, 0xA8, 0x68, 0xE8, 0x18, 0x98, 0x58, 0xD8, 0x38, 0xB8, 0x78, 0xF8,
        0x04, 0x84, 0x44, 0xC4, 0x24, 0xA4, 0x64, 0xE4, 0x14, 0x94, 0x54, 0xD4, 0x34, 0xB4, 0x74, 0xF4,
        0x0C, 0x8C, 0x4C, 0xCC, 0x2C, 0xAC, 0x6C, 0xEC, 0x1C, 0x9C, 0x5C, 0xDC, 0x3C, 0xBC, 0x7C, 0xFC,
        0x02, 0x82, 0x42, 0xC2, 0x22, 0xA2, 0x62, 0xE2, 0x12, 0x92, 0x52, 0xD2, 0x32, 0xB2, 0x72, 0xF2,
        0x0A, 0x8A, 0x4A, 0xCA, 0x2A, 0xAA, 0x6A, 0xEA, 0x1A, 0x9A, 0x5A, 0xDA, 0x3A, 0xBA, 0x7A, 0xFA,
        0x06, 0x86, 0x46, 0xC6, 0x26, 0xA6, 0x66, 0xE6, 0x16, 0x96, 0x56, 0xD6, 0x36, 0xB6, 0x76, 0xF6,
        0x0E, 0x8E, 0x4E, 0xCE, 0x2E, 0xAE, 0x6E, 0xEE, 0x1E, 0x9E, 0x5E, 0xDE, 0x3E, 0xBE, 0x7E, 0xFE,
        0x01, 0x81, 0x41, 0xC1, 0x21, 0xA1, 0x61, 0xE1, 0x11, 0x91, 0x51, 0xD1, 0x31, 0xB1, 0x71, 0xF1,
        0x09, 0x89, 0x49, 0xC9, 0x29, 0xA9, 0x69, 0xE9, 0x19, 0x99, 0x59, 0xD9, 0x39, 0xB9, 0x79, 0xF9,
        0x05, 0x85, 0x45, 0xC5, 0x25, 0xA5, 0x65, 0xE5, 0x15, 0x95, 0x55, 0xD5, 0x35, 0xB5, 0x75, 0xF5,
        0x0D, 0x8D, 0x4D, 0xCD, 0x2D, 0xAD, 0x6D, 0xED, 0x1D, 0x9D, 0x5D, 0xDD, 0x3D, 0xBD, 0x7D, 0xFD,
        0x03, 0x83, 0x43, 0xC3, 0x23, 0xA3, 0x63, 0xE3, 0x13, 0x93, 0x53, 0xD3, 0x33, 0xB3, 0x73, 0xF3,
        0x0B, 0x8B, 0x4B, 0xCB, 0x2B, 0xAB, 0x6B, 0xEB, 0x1B, 0x9B, 0x5B, 0xDB, 0x3B, 0xBB, 0x7B, 0xFB,
        0x07, 0x87, 0x47, 0xC7, 0x27, 0xA7, 0x67, 0xE7, 0x17, 0x97, 0x57, 0xD7, 0x37, 0xB7, 0x77, 0xF7,
        0x0F, 0x8F, 0x4F, 0xCF, 0x2F, 0xAF, 0x6F, 0xEF, 0x1F, 0x9F, 0x5F, 0xDF, 0x3F, 0xBF, 0x7F, 0xFF
    };
#elif defined(__AVX__)
    typedef __m256d Vec;
    Vec vec_init(double value)       { return _mm256_set1_pd(value); }
    bool vec_is_any_le(Vec v, Vec f) { Vec m = v<=f; return ! _mm256_testz_pd(m, m); }
    int vec_is_le(Vec v1, Vec v2)    { return _mm256_movemask_pd(v1 <= v2); }
    const uint8_t k_bit_rev[] =
    {
        0b0000, 0b1000, 0b0100, 0b1100, 0b0010, 0b1010, 0b0110, 0b1110,
        0b0001, 0b1001, 0b0101, 0b1101, 0b0011, 0b1011, 0b0111, 0b1111
    };
#elif defined(__SSE4_1__)
    typedef __m128d Vec;
    Vec vec_init(double value)       { return _mm_set1_pd(value); }
    bool vec_is_any_le(Vec v, Vec f) { __m128i m = __m128i(v<=f); return ! _mm_testz_si128(m, m); }
    int vec_is_le(Vec v1, Vec v2)    { return _mm_movemask_pd(v1 <= v2); }
    const uint8_t k_bit_rev[] = { 0b00, 0b10, 0b01, 0b11 };
#elif defined(__SSSE3__)
    typedef __m128d Vec;
    Vec vec_init(double value)       { return _mm_set1_pd(value); }
    bool vec_is_any_le(Vec v, Vec f) { return bool(_mm_movemask_pd(v<=f)); }
    int vec_is_le(Vec v1, Vec v2)    { return _mm_movemask_pd(v1 <= v2); }
    const uint8_t k_bit_rev[] = { 0b00, 0b10, 0b01, 0b11 };
#endif

    constexpr int k_vec_size = sizeof(Vec) / sizeof(double);

    // Return true iff all of 8 members of vector v1 is
    // NOT less than or equal to v2.
    bool vec_all_nle(const Vec* v1, Vec v2)
    {
        for ( auto i = 0; i < 8/k_vec_size; i++ ) {
            if ( vec_is_any_le(v1[i], v2) ) {
                return false;
            }
        }
        return true;
    }

    // Return 8 bit value with bits set iff cooresponding
    // member of vector value is less than or equal to limit.
    unsigned pixels(const Vec* value, Vec limit)
    {
        unsigned res = 0;
        for ( auto i = 0; i < 8/k_vec_size; i++ ) {
            res <<= k_vec_size;
            res |= k_bit_rev[vec_is_le(value[i], limit)];
        }
        return res;
    }

    //
    // Do one iteration of mandelbrot calculation for a vector of eight
    // complex values.  Using Vec to work with groups of doubles speeds
    // up computations.
    //
    void calcSum(Vec* real, Vec* imag, Vec* sum, const Vec* init_real, Vec init_imag)
    {
        for ( auto vec = 0; vec < 8/k_vec_size; vec++ ) {
            auto r2 = real[vec] * real[vec];
            auto i2 = imag[vec] * imag[vec];
            auto ri = real[vec] * imag[vec];

            sum[vec] = r2 + i2;

            real[vec]=r2 - i2 + init_real[vec];
            imag[vec]=ri + ri + init_imag;
        }
    }

    //
    // Do 50 iterations of mandelbrot calculation for a vector of eight
    // complex values.  Check occasionally to see if the iterated results
    // have wandered beyond the point of no return (> 4.0).
    //
    unsigned mand8(bool to_prune, const Vec* init_real, Vec init_imag)
    {
        Vec k4_0 = vec_init(4.0);
        Vec real[8 / k_vec_size];
        Vec imag[8 / k_vec_size];
        Vec sum[8 / k_vec_size];
        for ( auto k = 0; k < 8/k_vec_size; k++ ) {
            real[k] = init_real[k];
            imag[k] = init_imag;
        }

        if ( to_prune ) {
            // 4*12 + 2 = 50
            for ( auto j = 0; j < 12; j++ ) {
                for ( auto k = 0; k < 4; k++ ) {
                    calcSum(real, imag, sum, init_real, init_imag);
                }
                if ( vec_all_nle(sum, k4_0) ) {
                    return 0; // prune
                }
            }
            calcSum(real, imag, sum, init_real, init_imag);
            calcSum(real, imag, sum, init_real, init_imag);
        } else {
            // 6*8 + 2 = 50
            for ( auto j = 0; j < 8; j++ ) {
                for ( auto k = 0; k < 6; k++ ) {
                    calcSum(real, imag, sum, init_real, init_imag);
                }
            }
            calcSum(real, imag, sum, init_real, init_imag);
            calcSum(real, imag, sum, init_real, init_imag);
        }

        return pixels(sum, k4_0);
    }

} // namespace

int main(int argc, char ** argv)
{
    // get width/height from arguments

    auto wid_ht = 16000;
    if ( argc >= 2 ) {
        wid_ht = atoi(argv[1]);
    }

    // round up to multiple of 8
    wid_ht = -(-wid_ht & -8);
    auto width = wid_ht;
    auto height = wid_ht;

    // allocate memory for pixels.
    auto dataLength = height*(width>>3);
    auto pixels = new uint8_t[dataLength];

    // calculate initial x values, store in r0
    Vec r0[width / k_vec_size];
    double* r0_ = reinterpret_cast<double*>(r0);
    for ( auto x = 0; x < width; x++ ) {
        r0_[x] = 2.0 / width * x - 1.5;
    }

    // generate the bitmap

    // process 8 pixels (one byte) at a time
    #pragma omp parallel for schedule(guided)
    for ( auto y = 0; y < height; y++ ) {
        // all 8 pixels have same y value (iy).
        auto iy = 2.0 / height *  y - 1.0;
        Vec init_imag = vec_init(iy);
        auto rowstart = y*width/8;
        bool to_prune = false;
        for ( auto x = 0; x < width; x += 8 ) {
            auto res = mand8(to_prune, &r0[x/k_vec_size], init_imag);
            pixels[rowstart + x/8] = res;
            to_prune = ! res;
        }
    }

    // write the data
    printf("P4\n%d %d\n", width, height);
    fwrite(pixels, 1, dataLength, stdout);
    delete[] pixels;

    return 0;
}
