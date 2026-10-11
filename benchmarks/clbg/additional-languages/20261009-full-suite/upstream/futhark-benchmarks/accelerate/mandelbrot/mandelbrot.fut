-- Port of Accelerate's Mandelbrot example.  This can be run with futhark-bench.
--
-- ==
-- tags { futhark-c futhark-opencl }
-- compiled input {  800  600 -0.7f32 0f32 3.067f32 100 16f32 }
-- compiled input { 1000 1000 -0.7f32 0f32 3.067f32 100 16f32 }
-- compiled input { 2000 2000 -0.7f32 0f32 3.067f32 100 16f32 }
-- compiled input { 4000 4000 -0.7f32 0f32 3.067f32 100 16f32 }
-- compiled input { 8000 8000 -0.7f32 0f32 3.067f32 100 16f32 }

import "generic_mandelbrot"

module mandelbrot = mandelbrot f32

def main (screenX: i32)
         (screenY: i32)
         (xcentre: f32)
         (ycentre: f32)
         (width: f32)
         (depth: i32)
         (radius: f32) =
  mandelbrot.render_mandelbrot (i64.i32 screenX) (i64.i32 screenY) xcentre ycentre width depth radius
