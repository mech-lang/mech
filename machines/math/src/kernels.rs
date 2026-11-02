macro_rules! define_unary_f64_kernel_module {
    ($module:ident, $feature:literal, $kernel:path) => {
        #[cfg(any(feature = $feature, feature = "dynamic-module"))]
        pub mod $module {
            pub fn scalar_f64(input: f64) -> f64 {
                $kernel(input)
            }

            pub fn view_f64(input: &[f64], out: &mut [f64]) {
                for (src, dst) in input.iter().zip(out.iter_mut()) {
                    *dst = scalar_f64(*src);
                }
            }
        }
    };
}

macro_rules! define_binary_f64_kernel_module {
    ($module:ident, $feature:literal, $kernel:path) => {
        #[cfg(any(feature = $feature, feature = "dynamic-module"))]
        pub mod $module {
            pub fn scalar_f64(lhs: f64, rhs: f64) -> f64 {
                $kernel(lhs, rhs)
            }
        }
    };
}

define_unary_f64_kernel_module!(round, "round", libm::round);
define_unary_f64_kernel_module!(sqrt, "sqrt", libm::sqrt);
define_unary_f64_kernel_module!(floor, "floor", libm::floor);
define_unary_f64_kernel_module!(ceil, "ceil", libm::ceil);
define_unary_f64_kernel_module!(sin, "sin", libm::sin);
define_unary_f64_kernel_module!(cos, "cos", libm::cos);
define_unary_f64_kernel_module!(tan, "tan", libm::tan);
define_unary_f64_kernel_module!(asin, "asin", libm::asin);
define_unary_f64_kernel_module!(acos, "acos", libm::acos);
define_unary_f64_kernel_module!(atan, "atan", libm::atan);

define_binary_f64_kernel_module!(atan2, "atan2", libm::atan2);
