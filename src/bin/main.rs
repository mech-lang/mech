extern crate mech_core;
extern crate serde; // 1.0.68
#[macro_use]
extern crate serde_derive; // 1.0.68

use mech_core::{Core, Quantity, ToQuantity, QuantityMath, make_quantity};
use mech_core::{Bar, Aliases};

extern crate hashbrown;
use hashbrown::hash_map::HashMap;
use serde::*;
use serde::ser::{Serialize, Serializer, SerializeSeq, SerializeMap};

extern crate serde_json;

fn main() {
  /*
    let mut p = Bar{
      id: 200,
      x: Aliases::new(),
    };
    p.x.insert(10, 20);
    p.x.insert(100, 200);

    let serialized = serde_json::to_string(&p).unwrap();
    println!("{:?}", serialized);
    let bar: Result<Bar,_> = serde_json::from_str(&serialized);
    println!("{:?}", bar);*/
//problem child                 │49825176195110e0              │49825176195110e-11            │85e0                          │0e0             
  let x = make_quantity(282743338860,-9,0);
  let angle = make_quantity(180,0,0);
  let offset = make_quantity(49825176195110, -11, 0);
  let answer = x.divide(angle);
  println!("{:?}", answer.to_string());

}