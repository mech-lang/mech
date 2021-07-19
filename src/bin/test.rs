use std::collections::VecDeque;
use rayon::prelude::*;
use ndarray::{arr1, array, Array1, Array2, ArrayBase, Axis, DataMut, Data, Ix1, Ix2, ArrayView1, Zip};
use map_in_place::MapVecInPlace;


fn par_add_vs<A>(a: &mut ArrayBase<A,Ix1>, b: f64) 
where
  A: DataMut<Elem = f64>
{ 
  a.par_mapv_inplace(|x| x + b);
}

fn par_multiply_vs<A>(a: &mut ArrayBase<A,Ix1>, b: f64) 
where
  A: DataMut<Elem = f64>
{ 
  a.par_mapv_inplace(|x| x * b);
}

fn par_add_vv<A>(a: &mut ArrayBase<A,Ix1>, b: &ArrayBase<A,Ix1>) 
where
  A: DataMut<Elem = f64>
{ 
  Zip::from(a).and(b).par_for_each(|a, &b| {
    *a = *a + b;
  });
}

fn par_add_vv2<A>(out: &mut ArrayBase<A,Ix1>, lhs: &ArrayBase<A,Ix1>, rhs: &ArrayBase<A,Ix1>) 
where
  A: DataMut<Elem = f64>
{ 
  Zip::from(out).and(lhs).and(rhs).par_for_each(|out, &lhs, &rhs| {
    *out = lhs + rhs;
  });
}


fn par_less_than_vs<A,B>(ix: &mut ArrayBase<B,Ix1>, lhs: &ArrayBase<A,Ix1>, rhs: f64) 
where
  A: Data<Elem = f64>,
  B: DataMut<Elem = bool>
{ 
  Zip::from(ix).and(lhs).par_for_each(|ix, lhs| {
    *ix = *lhs < rhs;
  });
}

fn par_greater_than_vs<A,B>(ix: &mut ArrayBase<B,Ix1>, lhs: &ArrayBase<A,Ix1>, rhs: f64) 
where
  A: Data<Elem = f64>,
  B: DataMut<Elem = bool>
{ 
  Zip::from(ix).and(lhs).par_for_each(|ix, lhs| {
    *ix = *lhs > rhs;
  });
}

fn par_set_vs<A,B>(lhs: &mut ArrayBase<A,Ix1>, ix: &ArrayBase<B,Ix1>, rhs: f64) 
where
  A: DataMut<Elem = f64>,
  B: Data<Elem = bool>
{ 
  Zip::from(lhs).and(ix).par_for_each(|lhs, ix| {
    if *ix == true {
      *lhs = rhs;
    }
  });
}

fn main() {
  //const sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();
  const n: usize = 1e6 as usize;
  //for n in sizes {
    let mut y = arr1(&vec![1.0; n]);
    let mut vy = arr1(&vec![2.0; n]);
    let mut iy1 = arr1(&vec![false; n]);
    let mut iy2 = arr1(&vec![false; n]);
    let mut iyy = arr1(&vec![false; n]);

    let mut x = arr1(&vec![1.0; n]);
    let mut vx = arr1(&vec![2.0; n]);
    let mut ix1 = arr1(&vec![false; n]);
    let mut ix2 = arr1(&vec![false; n]);
    let mut ixy = arr1(&vec![false; n]);


    let mut total_time = VecDeque::new();
    for _ in 0..1000 {
      let start_ns = time::precise_time_ns();



      par_add_vv(&mut y, &vy);
      par_less_than_vs(&mut iy1, &y, 0.0);
      par_greater_than_vs(&mut iy2, &y, 500.0);
      par_set_vs(&mut y, &iy1, 0.0);
      par_set_vs(&mut y, &iy2, 500.0);
      par_multiply_vs(&mut y,-0.8);

      par_add_vv(&mut x, &vx);
      par_less_than_vs(&mut ix1, &x, 0.0);
      par_greater_than_vs(&mut ix2, &x, 500.0);
      par_set_vs(&mut x, &ix1, 0.0);
      par_set_vs(&mut x, &ix2, 500.0);
      par_multiply_vs(&mut x,-0.8);

      /*let y2 = par_add_vv(&y,&vy);
      let iy1 = par_less_than_vs(&y2,0.0);
      let iy2 = par_greater_than_vs(&y2,500.0);
      let y3 = par_set_vs(&y2,&iy1,0.0);
      let y4 = par_set_vs(&y3,&iy2,500.0);
      let neg_vy = par_multiply_vs(&vy,-0.8);
      let iy3 = par_or_vv(&iy1,&iy2);
      let vy2 = par_set_vv(&vy, &iy3, &neg_vy);*/


      let end_ns = time::precise_time_ns();
      let time = (end_ns - start_ns) as f64;
      total_time.push_back(time);
      if total_time.len() > 100 {
        total_time.pop_front();
      }
    }
    let average_time: f64 = total_time.iter().sum::<f64>() / total_time.len() as f64; 
    println!("{:0.2?}", 1.0 / (average_time / 1_000_000_000.0));
    //println!("{:?}", a.sum());
  //}
}

/*
// BASE
1e1 - 10504201.68Hz
1e2 - 6373486.30Hz
1e3 - 3408316.29Hz
1e4 - 59594.05Hz
1e5 - 14598.39Hz
1e6 - 363.17Hz
1e7 - 31.27Hz

// RAYON
1e1 - 78299.34Hz
1e2 - 43857.34Hz
1e3 - 30101.53Hz
1e4 - 17297.45Hz
1e5 - 8135.22Hz
1e6 - 456.35Hz

// NDARRAY IN PLACE
1e1 - 29411764.71Hz
1e2 - 9624639.08Hz
1e3 - 5861664.71Hz
1e4 - 511012.32Hz
1e5 - 40693.75Hz
1e6 - 2565.97Hz
1e7 - 116.51Hz

// NDARRAY IN PLACE PARALLEL
1e1 - 138094.84Hz
1e2 - 52806.67Hz
1e3 - 41284.44Hz
1e4 - 24468.60Hz
1e5 - 19147.55Hz
1e6 - 4799.01Hz
1e7 - 140.54Hz

*/