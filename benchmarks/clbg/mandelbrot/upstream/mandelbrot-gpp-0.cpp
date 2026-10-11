// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// Contributed by Markus Flad.
// Unlike the original version, this version uses std::array to make the
// compiler vectorize.
//
// Compile with following g++ flags
//  -std=c++17 -O3 -Wall -march=native -mno-fma

#include <string>
#include <iostream>
#include <vector>
#include <complex>
#include <algorithm>
#include <thread>
#include <climits>
#include <version>
#include <array>

const auto numberOfCpuCores = std::thread::hardware_concurrency();

// The PortableBinaryBitmap manages access to the pbm output file and provides
// interlaced canvases that allow threads to write to the bitmap in parallel.
class PortableBinaryBitmap {
public:
    using Size=std::size_t;
    PortableBinaryBitmap(std::ostream& ostr, Size width, Size height)
    : _ostr (ostr)
    , _width (roundToMultiple(width, CHAR_BIT))
    , _height (roundToMultiple(height, numberOfCpuCores))
    , _data ((_width * _height) / CHAR_BIT) {
        _ostr << "P4" << '\n';
        _ostr << _width << ' ' << _height << '\n';
    }
    ~PortableBinaryBitmap() {
        _ostr.write(_data.data(), _data.size());
    }
    Size width() const noexcept {
        return _width;
    }
    Size height() const noexcept {
        return _height;
    }
    Size widthInBytes() const noexcept {
        return _width / CHAR_BIT;
    }
    struct Line {
        constexpr static Size pixelsPerWrite() noexcept {
            return CHAR_BIT;
        }
        Size y;
        Size width;
        char* data;
    };
    // The InterlacedCanvas provides interlaced access to the bitmap data. Each
    // thread must use its own InterlacedCanvas to write to the bitmap.
    class InterlacedCanvas {
    public:
        class Iterator {
        public:
            Iterator(Size y, Size _width, char* data,
                    Size interlaceIncrement, Size dataPointerIncrement) noexcept
            : _il {y, _width, data}
            , _interlaceIncrement (interlaceIncrement)
            , _dataPointerIncrement (dataPointerIncrement) {
            }
            Line& operator*() noexcept {
                return _il;
            }
            bool operator!=(const Iterator& other) const noexcept {
                return _il.data != other._il.data;
            }
            Iterator& operator++() noexcept {
                _il.y += _interlaceIncrement;
                _il.data += _dataPointerIncrement;
                return *this;
            }
        private:
            Line _il;
            Size _interlaceIncrement;
            Size _dataPointerIncrement;
        };
        InterlacedCanvas(PortableBinaryBitmap& pbm, Size yStart, Size increment)
                noexcept
        : _pbm (pbm)
        , _yStart (yStart)
        , _increment (increment)
        , _dataStart (yStart * pbm.widthInBytes())
        , _dataPointerIncrement (increment * pbm.widthInBytes()) {
        }
        Size width() const noexcept {
            return _pbm.width();
        }
        Size height() const noexcept {
            return _pbm.height();
        }
        Iterator begin() noexcept {
            return Iterator(_yStart, _pbm.width(),
                    _pbm._data.data() + _dataStart,
                    _increment, _dataPointerIncrement);
        }
        Iterator end() noexcept {
            return Iterator(_yStart + _pbm.height(), _pbm.width(),
                    _pbm._data.data() + _pbm._data.size() + _dataStart,
                    _increment, _dataPointerIncrement);
        }
    private:
        PortableBinaryBitmap& _pbm;
        Size _yStart;
        Size _increment;
        Size _dataStart;
        Size _dataPointerIncrement;
    };
    std::vector<InterlacedCanvas> provideInterlacedCanvas(Size increment)
            noexcept {
        std::vector<InterlacedCanvas> interlacedCanvasVector;
        for (Size yStart=0; yStart<increment; ++yStart) {
            interlacedCanvasVector.emplace_back(*this, yStart, increment);
        }
        return interlacedCanvasVector;
    }
    static Size roundToMultiple (Size number, Size base) noexcept {
        return number + ((number % base) ? (base - number % base) : 0);
    }
private:
    std::ostream& _ostr;
    Size _width;
    Size _height;
    std::vector<char> _data;
};

template<typename NUMBER_TYPE>
class VectorizedNumber
{
public:
    constexpr static std::size_t SIZE = 8;
    using NumericArray = std::array<NUMBER_TYPE, SIZE>;

    VectorizedNumber() noexcept
    : _x(_values.data()) {
    }
    explicit VectorizedNumber(NUMBER_TYPE value) noexcept
    : _x(_values.data()) {
        std::fill(_values.begin(), _values.end(), value);
    }
    VectorizedNumber(const VectorizedNumber& other) noexcept
    : _x(_values.data()) {
        *this = other;
    }
    VectorizedNumber& operator=(const VectorizedNumber& other) noexcept {
        // In GCC (unlike Clang), using the std::array assignment operator and
        // also its copy constructor slows down SIMD performance. Therefore we
        // use a raw loop here.
        for (size_t i=0; i<SIZE; ++i) {
            _values[i] = other._values[i];
        }
        return *this;
    }
    NUMBER_TYPE operator[](std::size_t i) const noexcept{
        return _values[i];
    }
    NUMBER_TYPE& operator[](std::size_t i) noexcept {
        return _values[i];
    }
    typename NumericArray::const_iterator begin() const noexcept {
        return _values.begin();
    }
    typename NumericArray::const_iterator end() const noexcept {
        return _values.end();
    }
    bool operator>(NUMBER_TYPE value) const noexcept {
        return (std::all_of(_values.begin(), _values.end(),
                [&value](NUMBER_TYPE v) {return v > value;}));
    }
    char lteToPixels(NUMBER_TYPE threshold) const noexcept {
        char result = 0;
        if (_values[0] <= threshold) result |= 0b10000000;
        if (_values[1] <= threshold) result |= 0b01000000;
        if (_values[2] <= threshold) result |= 0b00100000;
        if (_values[3] <= threshold) result |= 0b00010000;
        if (_values[4] <= threshold) result |= 0b00001000;
        if (_values[5] <= threshold) result |= 0b00000100;
        if (_values[6] <= threshold) result |= 0b00000010;
        if (_values[7] <= threshold) result |= 0b00000001;
        return result;
    }
private:
    NumericArray _values;
    // A pointer to the data, which is not used outside, but helps the compiler
    // with the SIMD optimization (especially GCC 11).
    NUMBER_TYPE* _x;
};

// VectorizedComplex provides a convenient interface to deal with complex
// numbers and uses the power of SIMD for high execution speed.
template <typename NUMBER_TYPE>
class VectorizedComplex {
public:
    using Size = std::size_t;

    VectorizedComplex() = default;
    VectorizedComplex(const VectorizedComplex&) = default;
    VectorizedComplex& operator=(const VectorizedComplex&) = default;
    VectorizedComplex(const VectorizedNumber<NUMBER_TYPE>& reals,
            NUMBER_TYPE commonImagValue) noexcept
    : _reals(reals)
    , _imags(commonImagValue){
    }
    VectorizedComplex& squareAndAdd(const VectorizedComplex& c,
            VectorizedNumber<NUMBER_TYPE>& squaredAbs) noexcept {
        for (Size i=0; i<VectorizedNumber<NUMBER_TYPE>::SIZE; ++i) {
            auto realSquared = _reals[i] * _reals[i];
            auto imagSquared = _imags[i] * _imags[i];
            auto realTimesImag = _reals[i] * _imags[i];
            _reals[i] = realSquared - imagSquared + c._reals[i];
            _imags[i] = realTimesImag + realTimesImag + c._imags[i];
            squaredAbs[i] = realSquared + imagSquared;
        }
        return *this;
    }
private:
    VectorizedNumber<NUMBER_TYPE> _reals;
    VectorizedNumber<NUMBER_TYPE> _imags;
};

// The ComplexPlaneCalculator performs function f(c), with c as a
// VectorizedComplex and a byte as the return value. Due to its eightfold
// vectorization, each returned bit can return a Boolean value from the
// calculation f(c). The full byte is then written to the canvas. This is done
// until the whole bitmap is filled.
template <typename NUMBER_TYPE, class Functor>
class ComplexPlaneCalculator {
public:
    using VComplex = VectorizedComplex<NUMBER_TYPE>;
    using Line = typename PortableBinaryBitmap::Line;
    using Size = std::size_t;

    ComplexPlaneCalculator(const std::complex<NUMBER_TYPE>& cFirst,
            const std::complex<NUMBER_TYPE>& cLast,
            PortableBinaryBitmap::InterlacedCanvas& canvas, Functor f) noexcept
    : _cFirst(cFirst)
    , _cLast(cLast)
    , _canvas(canvas)
    , _f(f) {
    }
    void operator()() noexcept {
        const NUMBER_TYPE realRange = _cLast.real() - _cFirst.real();
        const NUMBER_TYPE imagRange = _cLast.imag() - _cFirst.imag();
        const NUMBER_TYPE rasterReal = realRange / _canvas.width();
        const NUMBER_TYPE rasterImag = imagRange / _canvas.height();
        std::vector<VectorizedNumber<NUMBER_TYPE>> cRealValues;
        cRealValues.reserve(_canvas.width() / Line::pixelsPerWrite());
        for (Size x=0; x<_canvas.width(); x+=Line::pixelsPerWrite()) {
            VectorizedNumber<NUMBER_TYPE> cReals;
            for (Size i=0; i<Line::pixelsPerWrite(); ++i) {
                cReals[i] = _cFirst.real() + (x+i)*rasterReal;
            }
            cRealValues.push_back(cReals);
        }
        for (Line& line : _canvas) {
            char* nextPixels = line.data;
            char lastPixels = 0x00;
            const NUMBER_TYPE cImagValue = _cFirst.imag() + line.y*rasterImag;
            for (const VectorizedNumber<NUMBER_TYPE>& cReals : cRealValues) {
                const VComplex c(cReals, cImagValue);
                *nextPixels = _f(c, lastPixels);
                lastPixels = *nextPixels;
                nextPixels++;
            }
        }
    }
private:
    std::complex<NUMBER_TYPE> _cFirst;
    std::complex<NUMBER_TYPE> _cLast;
    PortableBinaryBitmap::InterlacedCanvas _canvas;
    Functor _f;
};

// Functor calculating a Mandelbrot iteration for a VectorizedComplex. This
// means that for eight complex numbers the Mandelbrot calculation is
// (potentially) executed in parallel. The result is a byte that contains a 1
// for each bit if the corresponding complex number is in the Mandelbrot set,
// and a 0 if it is not.
template <typename NUMBER_TYPE>
class MandelbrotFunction {
public:
    using VComplex = VectorizedComplex<NUMBER_TYPE>;
    using Size = std::size_t;
    constexpr static Size ITERATIONS_WITHOUT_CHECK = 5;
    constexpr static char NONE_IN_MANDELBROT_SET = 0x00;

    MandelbrotFunction(Size maxIterations, NUMBER_TYPE pointOfNoReturn = 2.0)
            noexcept
    : _maxOuterIterations(maxIterations / ITERATIONS_WITHOUT_CHECK - 2)
    , _squaredPointOfNoReturn(pointOfNoReturn * pointOfNoReturn) {
    }
    static void doMandelbrotIterations(VComplex& z, const VComplex& c,
            VectorizedNumber<NUMBER_TYPE>& squaredAbs) noexcept {
        for (Size j=0; j<ITERATIONS_WITHOUT_CHECK; ++j) {
            z.squareAndAdd(c, squaredAbs);
        }
    }
    char operator()(const VComplex& c, char lastPixels) const noexcept {
        VComplex z = c;
        VectorizedNumber<NUMBER_TYPE> squaredAbs;
        if (lastPixels == NONE_IN_MANDELBROT_SET) {
            for (Size i=0; i<_maxOuterIterations; ++i) {
                doMandelbrotIterations(z, c, squaredAbs);
                if (squaredAbs > _squaredPointOfNoReturn) {
                    return NONE_IN_MANDELBROT_SET;
                }
            }
        } else {
            for (Size i=0; i<_maxOuterIterations; ++i) {
                doMandelbrotIterations(z, c, squaredAbs);
            }
        }
        doMandelbrotIterations(z, c, squaredAbs);
        doMandelbrotIterations(z, c, squaredAbs);
        return squaredAbs.lteToPixels(_squaredPointOfNoReturn);
    }
private:
    Size _maxOuterIterations;
    NUMBER_TYPE _squaredPointOfNoReturn;
};

int main(int argc, char** argv) {
    using NumberType = double;
    using ComplexNumber = std::complex<NumberType>;
    using MandelbrotCalculator = ComplexPlaneCalculator<NumberType,
            MandelbrotFunction<NumberType>>;
    std::size_t n = 16000;
    if (argc>=2) {
        n = atoi(argv[1]);
    }
    const std::size_t maxIterations = 50;
    PortableBinaryBitmap pbm(std::cout, n, n);
    auto canvasVector = pbm.provideInterlacedCanvas(numberOfCpuCores);
    std::vector<std::thread> threads;
    for (auto& canvas : canvasVector) {
        threads.emplace_back(MandelbrotCalculator (ComplexNumber(-1.5, -1.0),
                ComplexNumber(0.5, 1.0), canvas,
                MandelbrotFunction<NumberType> (maxIterations)));
    }
    for (auto& t : threads) {
        t.join();
    }
    return 0;
}
