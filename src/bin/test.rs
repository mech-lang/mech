
use map_in_place::MapVecInPlace;
use std::collections::VecDeque;
use rayon::prelude::*;
use ndarray::arr1;

fn main() {
    let sizes: Vec<usize> = vec![1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7].iter().map(|x| *x as usize).collect();

    for n in sizes {
        let mut v: Vec<f64> = vec![1.0; n];
        let mut a = arr1(&vec![1.0; n]);


        let mut total_time = VecDeque::new();
        for _ in 0..4000 {
            let start_ns = time::precise_time_ns();

            a.par_mapv_inplace(|x| x + 1.0);

            //v = v.map(|n| n + 1.0);
            //v = v.iter().map(|n| n + 1).collect::<Vec<f64>>();
            //v = v.par_iter().map(|n| n + 1 ).collect::<Vec<f64>>();

            let end_ns = time::precise_time_ns();
            let time = (end_ns - start_ns) as f64;
            total_time.push_back(time);
            if total_time.len() > 1000 {
            total_time.pop_front();
            }
        }
        let average_time: f64 = total_time.iter().sum::<f64>() / total_time.len() as f64; 
        println!("{:e} - {:0.2?}Hz", n, 1.0 / (average_time / 1_000_000_000.0));
        //println!("{:?}", a.sum());
    }
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