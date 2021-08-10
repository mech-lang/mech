use arrayfire::*;

fn main() {

  let num_rows: u64 = 5;
  let num_cols: u64 = 3;
  let dims = Dim4::new(&[num_rows, num_cols, 1, 1]);
  let a = randu::<f32>(dims);
  af_print!("Create a 5-by-3 matrix of random floats on the GPU", a);

}