extern crate mech_syntax;
extern crate mech_core;
#[macro_use]
extern crate lazy_static;
use mech_syntax::compiler::Compiler;
use std::cell::RefCell;
use std::rc::Rc;
use mech_core::*;

lazy_static! {
  static ref TXN: Vec<Change> = vec![Change::Set((hash_str("x"), vec![(TableIndex::Index(1), TableIndex::Index(1), Value::F32(F32::new(9.0)))]))];
  static ref TXN2: Vec<Change> = vec![
    Change::Set((hash_str("x"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::F32(F32::new(9.0)))]))
  ];
  static ref TXN3: Vec<Change> = vec![
    Change::Set((hash_str("x"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::F32(F32::new(9.0)))])),
    Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::F32(F32::new(1.0)))])),
  ];
  static ref TXN4: Vec<Change> = vec![
    Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::F32(F32::new(1.0)))])),
    Change::Set((hash_str("time/timer"), vec![(TableIndex::Index(1), TableIndex::Index(2), Value::F32(F32::new(2.0)))])),
  ];
}

macro_rules! test_mech {
  ($func:ident, $input:tt, $test:expr) => (
    #[test]
    fn $func() -> Result<(),MechError> {
      let mut compiler = Compiler::new();
      let mut core = Core::new();

      let input = String::from($input);
      let blocks = compiler.compile_str(&input)?;
      
      for block in blocks {
        core.insert_block(Rc::new(RefCell::new(block)))?;
      }

      let test: Value = $test;
      let actual = core.get_table("test")?.borrow().get_raw(0, 0);
      match actual {
        Ok(value) => {
          assert_eq!(value,test);
        },
        _ => {assert!(false)},
      }
      Ok(())
    }
  )
}

macro_rules! test_mech_txn {
  ($func:ident, $input:tt, $txn:tt, $test:expr) => (
    #[test]
    fn $func() -> Result<(),MechError> {
      let mut compiler = Compiler::new();
      let mut core = Core::new();

      let input = String::from($input);
      let blocks = compiler.compile_str(&input)?;
      
      core.insert_blocks(blocks)?;
      
      core.schedule_blocks()?;

      core.process_transaction(&$txn)?;

      let test: Value = $test;
      let actual = core.get_table("test")?.borrow().get_raw(0, 0);
      match actual {
        Ok(value) => {
          assert_eq!(value,test);
        },
        _ => {assert!(false)},
      }
      Ok(())
    }
  )
}

// ## Constant

test_mech!(constant_float_basic, "#test = 5.5",Value::F32(F32::new(5.5)));

test_mech!(constant_float_leading_zero, "#test = 0.5",Value::F32(F32::new(0.5)));
test_mech!(constant_float_leading_decimal, "#test = .5",Value::F32(F32::new(0.5)));

test_mech!(constant_basic, "block
  #test = 5",Value::F32(F32::new(5.0)));

  test_mech!(constant_empty_table, "
block
  #test = _", Value::Empty);

test_mech!(constant_inline_empty, "#test = [first: 12, second: _, third: 45]",Value::F32(F32::new(12.0)));

test_mech!(constant_hex, "#test = 0xABC123",Value::U128(U128::new(11256099)));

// ## Unicode

test_mech!(unicode, "
block 
  😃 = 1
  🤦🏼‍♂️ = 2
  y̆és = 🤦🏼‍♂️ + 😃
  #test = y̆és",Value::F32(F32::new(3.0)));

// ## Table

test_mech!(table_define_inline_expressions, "
block
  #x = [x: 1 + 2, y: 2 + 2]
block
  #test = #x.x + #x.y", Value::F32(F32::new(7.0)));

test_mech!(table_inline_multirow, r#"
block
  #x = [x: 1
        y: 2
        z: 3]
block
  #test = #x.x + #x.y + #x.z"#, Value::F32(F32::new(6.0)));

test_mech!(table_inline_multirow_nested, r#"
block
  #x = [
    root: "mech-root"
    contains: [
      type: "a",
      href: "foo.bar",
      contains: 10
    ]
  ]
block
  #test = #x.contains.contains"#, Value::F32(F32::new(10.0)));

test_mech!(table_anonymous_table_trailing_whitespace, "
block
  #test = [|d|
            5  ]", Value::F32(F32::new(5.0)));

test_mech!(table_anonymous_table_trailing_newline, "
block
  #test = [|d|
            5  
          ]", Value::F32(F32::new(5.0)));

test_mech!(table_define_empty_table, "
block
  #bots = [|name position|]
block
  #bots += [position: 4 name: 2]
block
  #test = #bots.position / #bots.name", Value::F32(F32::new(2.0)));

test_mech!(table_define_program, "# A Working Program

## Section Two

  #test = 9", Value::F32(F32::new(9.0)));

test_mech!(table_multi_line_inline, "
block
  #x = [
    x: 1
    y: 2
    z: 3
  ]
block
  #test = #x.x + #x.y + #x.z", Value::F32(F32::new(6.0)));

test_mech!(table_size, "
block
  #x = [1 2
        3 4
        5 6]
block
  #y = table/size(table: #x)

block
  #test = #y{1} + #y{2}", Value::U64(U64::new(5)));

// ## Select

test_mech!(select_table,"  
block
  #x = 123
block
  #test = #x", Value::F32(F32::new(123.0)));

// ## Math

test_mech!(math_constant,"#test = 10", Value::F32(F32::new(10.0)));

test_mech!(math_add,"#test = 1 + 1", Value::F32(F32::new(2.0)));

test_mech!(math_add_u16,"#test = 10<u16> + 400<u16>", Value::U16(U16::new(410)));

//test_mech!(math_add_u8_u16,"#test = 10<u8> + 400<u16>", Value::U16(410)));

test_mech!(math_add_f32,"#test = 123.456 + 456.123", Value::F32(F32::new(579.579)));

test_mech!(math_add_m_km,"#test = 400<m> + 1<km>", Value::Length(F32::new(1400.0)));

test_mech!(math_add_ms_s,"#test = 4<s> + 100<ms>", Value::Time(F32::new(4100.0)));

test_mech!(math_subtract,"#test = 3 - 1", Value::F32(F32::new(2.0)));

test_mech!(math_multiply,"#test = 2 * 2", Value::F32(F32::new(4.0)));

test_mech!(math_divide,"#test = 4 / 2", Value::F32(F32::new(2.0)));

test_mech!(math_two_terms,"#test = 1 + 2 * 9", Value::F32(F32::new(19.0)));

test_mech!(math_constant_collision,"#test = 123 + 1", Value::F32(F32::new(124.0)));

test_mech!(math_subtract_columns,"#test = stats/sum(column: [5;6;7] - [1;2;3])", Value::F32(F32::new(12.0)));

test_mech!(math_multiple_variable_graph,"block
  a = z * 5
  d = 9 * z
  z = 5
  #test = d * z + a", Value::F32(F32::new(250.0)));

test_mech!(math_multiple_variable_graph_new_ordering,"block
  a = z * 5
  z = 5
  d = 9 * z
  #test = d * z + a", Value::F32(F32::new(250.0)));

  test_mech!(math_add_columns_alias,"
block
  x = 1:10
  y = 1:10
  #ball = [|x y|
            x y]
block
  #test = stats/sum(column: #ball.x + #ball.y)", Value::F32(F32::new(110.0)));

  test_mech!(math_add_columns_indices,"
block
  x = 1:10
  y = 1:10
  #ball = [|x y|
            x y]
block
  #test = stats/sum(column: #ball{:,1} + #ball{:,2})", Value::F32(F32::new(110.0)));

test_mech!(math_on_whole_table,"
block
  #x = 200
block
  #test = #x + 5", Value::F32(F32::new(205.0)));

test_mech!(select_column_by_id,"  
block
  #ball = [x: 56 y: 2 vx: 3 vy: 4]
block
  #test = #ball.y", Value::F32(F32::new(2.0)));

test_mech!(math_multiple_rows_select,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = #ball.x + #ball.y * #ball.vx", Value::F32(F32::new(177.0)));

test_mech!(math_const_and_select,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = 9 + #ball.x", Value::F32(F32::new(24.0)));

test_mech!(math_select_and_const,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = #ball.x + 9", Value::F32(F32::new(24.0)));

test_mech!(partial_bouncing_ball,"# Bouncing Balls
Define the environment
  #ball = [x: 15 y: 9 vx: 18 vy: 9]
  #time/timer = [period: 10]
  #gravity = 10

Now update the block positions
  x = #ball.x + #ball.vx
  y = #ball.y + #ball.vy
  dt = #time/timer.period
  #test = x + y * dt", Value::F32(F32::new(213.0)));

test_mech!(math_add_columns,"
block
  #ball = [|x y|
            1 2
            3 4
            5 6]
block
  #test = stats/sum(column: #ball.x + #ball.y)", Value::F32(F32::new(21.0)));

test_mech!(math_add_matrices,"
block
  x = [1 2
       4 5]
  y = [10 11
       13 14]
  z = x + y
  #test = z{1} + z{2} + z{3} + z{4}", Value::F32(F32::new(60.0)));

test_mech!(math_scalar_plus_vector,"
block
  x = 3:6
  y = 5 + x
  #test = y{1} + y{2} + y{3} + y{4}", Value::F32(F32::new(38.0)));

test_mech!(math_vector_plus_scalar_inline,"
block
  #x = [1 2 3] + 1
  
block
  #test = #x{1} + #x{2} + #x{3}", Value::F32(F32::new(9.0)));

test_mech!(math_vector_plus_scalar_inline_reverse,"
block
  #x = 1 + [1 2 3]
    
block
  #test = #x{1} + #x{2} + #x{3}", Value::F32(F32::new(9.0)));

test_mech!(math_vector_plus_scalar,"
block
  x = [1 2 3]
  #x = x + 1

block
  #test = #x{1} + #x{2} + #x{3}", Value::F32(F32::new(9.0)));

test_mech!(math_parenthetical_expression_constants,"
block
  #test = (1 + 2) * 3", Value::F32(F32::new(9.0)));

// ## Ranges

test_mech!(range_basic,r#"
block
  #range = 5 : 14
block
  #test = stats/sum(column: #range)"#, Value::F32(F32::new(95.0)));

test_mech!(range_and_cat,r#"
block
  x = 1:4
  y = 1:4
  #ball = [x y]
block
  #test = stats/sum(table: #ball)"#, Value::F32(F32::new(20.0)));

// ## Subscripts

test_mech!(subscript_scalar_math,"
block
  x = 3:6
  y = 10:12
  #test = x{1,1} + y{3,1}", Value::F32(F32::new(15.0)));

test_mech!(subscript_scan,"
block
  x = 10:20
  z = 3
  #test = x{z}", Value::F32(F32::new(12.0)));

test_mech!(subscript_single_horz,"
block
  x = [1 2 3]
  #test = x{2}", Value::F32(F32::new(2.0)));

test_mech!(subscript_single_vert,"
block
  x = [1; 2; 3]
  #test = x{2}", Value::F32(F32::new(2.0)));

// ## Comparators

test_mech!(compare_greater_than,"#test = 16 > 15", Value::Bool(true));
test_mech!(compare_less_than,"#test = 16 < 15", Value::Bool(false));

test_mech!(compare_greater_than_equal,"
block
  #x = [1; 2; 3]
  #y = [2; 1; 3]
  
block
  ix = #x >= #y
  #test = stats/sum(column: #x{ix})", Value::F32(F32::new(5.0))); 

test_mech!(compare_less_than_equal,"
block
  #x = [1; 2; 3]
  #y = [2; 1; 3]
  
block
  ix = #x <= #y
  #test = stats/sum(column: #x{ix})", Value::F32(F32::new(4.0))); 


test_mech!(compare_equal,"
block
  #x = [1; 2; 3; 2]
  #y = [2; 1; 3; 2]
  
block
  ix = #x == #y
  #test = stats/sum(column: #x{ix})", Value::F32(F32::new(5.0))); 

test_mech!(compare_equal_boolean,"
block
  #x = [true; true; true; true]
  #y = [true; true; true; true]
  
block
  ix = #x == #y
  #test = set/all(column: ix)", Value::Bool(true)); 

test_mech!(compare_not_equal_boolean,"
block
  #x = [true; true; true; true]
  #y = [false; false; false; false]
  
block
  ix = #x != #y
  #test = set/all(column: ix)", Value::Bool(true)); 

test_mech!(compare_equal_string,r#"
block
  #x = [1; 2; 3; 4]
  #y = ["a"; "b"; "a"; "b"]
  
block
  ix = #y == "a"
  #test = stats/sum(column: #x{ix})"#, Value::F32(F32::new(4.0))); 

test_mech!(compare_not_equal,"
block
  #x = [1; 2; 3; 2]
  #y = [2; 1; 3; 2]
  
block
  ix = #x != #y
  #test = stats/sum(column: #x{ix})", Value::F32(F32::new(3.0))); 

// ## Set

test_mech!(set_column_simple,"
block
  #test = [|x|
            9]
block
  #test.x := 77", Value::F32(F32::new(77.0)));

test_mech!(set_empty_with_index,"
block
  #foo = [|x y|]

block
  #foo := [|x y|
          true  1
          false 2
          true  3]
block
  ix = #foo.x == true
  #foo.y{ix} := 10

block
  #test = stats/sum(column: #foo.y)", Value::F32(F32::new(22.0)));

test_mech!(set_multirow_empty,"
block
  #x = [|x y|]

Define the environment
  #x := [|x y| 1 2; 3 4]

block
  #test = #x.x{1} + #x.x{2} + #x.y{1} + #x.y{2}", Value::F32(F32::new(10.0)));

test_mech!(set_column_alias,"
block
  #ball = [x: 0 y: 0]

block
  #foo = [x: 100 y: 120]
  #z = 100

block
  #foo.x := 200

block
  #ball.x := #foo.x
  
block
  #test = #ball.x", Value::F32(F32::new(200.0)));


test_mech!(set_single_index,"
block
  #x = [200; 0; 0]
 
block 
  #x{3} := 7

block
  #test = stats/sum(column: #x)", Value::F32(F32::new(207.0)));


test_mech!(set_single_index_math,"
block
  #x = [1;2;3]
  
block
  #x{2,1} := 10

block
  y = #x * 2
  #test = stats/sum(column: y)", Value::F32(F32::new(28.0)));

test_mech!(set_logical_false,"
block
  #x = [1; 2; 3]
  #clicked = [false; false; false]

block
  #ball = [x: #x]

block
  #ball{#clicked} := 10
  
block
  #test = stats/sum(column: #ball)", Value::F32(F32::new(6.0)));

test_mech!(set_column_logical,"
block
  #q = [|x|
         1
         4
         7]
block
  x = #q.x
  ix = x > 1
  #q.x{ix} := 10

block
  #test = #q.x{1} + #q.x{2} + #q.x{3}", Value::F32(F32::new(21.0)));

test_mech!(set_second_column_logical,"
block
  #ball = [|x y z|
            1 2 3
            4 5 6
            7 8 9]
block
  x = #ball.y
  ix = x > 5
  #ball.y{ix} := 3
block
  #test = #ball.y{1} + #ball.y{2} + #ball.y{3}", Value::F32(F32::new(10.0)));

test_mech!(set_second_omit_row_subscript,"
block
  #ball = [x: 15 y: 9 vx: 40 vy: 9]
  #time/timer = [period: 15 tick: 0]
  #gravity = 2

block
  ~ #time/timer.tick
  #ball.y := #ball.vy + #gravity

block
  #test = #ball.y", Value::F32(F32::new(11.0)));

test_mech!(set_rhs_math_filters_logic,"
block
  #ball = [|x y  vx vy|
            1 2  3  4
            5 6  7  8
            9 10 11 12]

block
  ix = #ball.vy > 10
  iy = #ball.vy < 5
  ixx = ix | iy
  #ball.y{ixx} := #ball.vy * 2

block
  #test = #ball{1,2} + #ball{3,2}", Value::F32(F32::new(32.0)));

test_mech!(set_implicit_logic,"
block
  #ball = [|x y vx vy|
            1 2 3 4
            5 6 7 8
            9 10 11 12]
  #time/timer = [period: 15 tick: 0]
  #gravity = 2

block
  ix = #ball.vy > 10
  iy = #ball.vy < 5
  #ball.y{ix | iy} := #ball.vy * 2

block
  #test = #ball{1,2} + #ball{3,2}", Value::F32(F32::new(32.0)));

test_mech!(set_inline_row,"
block
  #launch-point = [x: 0 y: 0]
block
  #launch-point := [x: 10 y: 20]
block
  #test = #launch-point.x + #launch-point.y", Value::F32(F32::new(30.0)));

test_mech!(set_empty_table,"
block
  #x = []
block
  #x := [10 20; 30 40]
block
  #test = stats/sum(table: #x)", Value::F32(F32::new(100.0)));

test_mech!(set_empty_table_with_column,"
block
  #x = [|code|]
block
  #x := 10  
block
  #test = #x", Value::F32(F32::new(10.0)));

test_mech!(set_table_index_row_dependency,"
block
  #x = [x: 3]

block
  #balls = [x: 10]

block
  #clicked = true

block
  ~ #x
  #balls.x{#clicked} := #x.x
  
block
  #test = #balls.x", Value::F32(F32::new(3.0)));

// ## Concat

test_mech!(concat_horzcat_data,"
block
  x = 1:10
  y = 11:20
  #z = [x y]
  
block
  #test = #z{1,1} + #z{1,2} + #z{2,1} + #z{1,1}", Value::F32(F32::new(15.0)));

test_mech!(concat_horzcat_autofill,r#"
block
  x = ["a"; "b"; "c"; "d"]
  #y = [type: 1 class: "table" result: x]
  
block
  #test = stats/sum(column: #y.type)"#, Value::F32(F32::new(4.0)));

// ## Append

test_mech!(append_row_empty,"
block
  #robot = [|name position|]
  
block
  #robot += [name: 10 position: 20]
  
block
  #test = #robot.name + #robot.position", Value::F32(F32::new(30.0)));

test_mech!(append_row_inline,"
block
  #foo = [|x y z|
           5 6 7]

block
  #foo += [x: 100 y: 110 z: 120]
  
block
  ix = #foo.x > 50
  #test = #foo{ix, :}", Value::F32(F32::new(100.0)));

test_mech!(append_row_expression,"
block
  #x = 20
block
  #x += 10
block
  #test = stats/sum(column: #x)", Value::F32(F32::new(30.0)));

test_mech!(append_row_math,"
block
  #x = 20
block
  #x += 5 * 2
block
  #test = stats/sum(column: #x)", Value::F32(F32::new(30.0)));

test_mech!(append_row_math_empty,"
block
  #x = []
block
  #x += 5 * 2
block
  #test = stats/sum(column: #x)", Value::F32(F32::new(10.0)));

test_mech!(append_row_math_empty_named," 
block
  #x = [|x|]
block
  #x += 5 * 2
block
  #test = stats/sum(column: #x)", Value::F32(F32::new(10.0)));  

test_mech!(append_row_math_empty_whole_table,"
block
  #x = []
block
  x = 10 + 20
  #x += x
block
  #test = #x", Value::F32(F32::new(30.0)));

test_mech!(append_row_select_linear_range,"
block
  #x = [10 20; 30 40;]
block
  x = [10 20 30]
  #x += x{1:2}
block
  #test = stats/sum(table: #x)", Value::F32(F32::new(130.0)));  

test_mech!(append_row_select_linear,"
block
  #x = [10; 30]
block
  x = [10; 20; 30]
  #x += x{2}  
block
  #test = stats/sum(column: #x)", Value::F32(F32::new(60.0))); 

test_mech!(append_multiple_rows,"
block
  #x = [|x y|
         1 2]
block
  #x += [|x y|
          3 4
          5 6]
block
  #test = stats/sum(table: #x)", Value::F32(F32::new(21.0))); 
  

test_mech!(append_arbitrary_types_x,"
block
  #x = [|x y z|]

block
  #x += [y: 10, x: 99<u64>]

block
  #test = #x.x", Value::U64(U64::new(99))); 
    
test_mech!(append_arbitrary_types_y,"
block
  #x = [|x y z|]

block
  #x += [y: 10, x: 99<u64>]

block
  #test = #x.y", Value::F32(F32::new(10.0))); 

// ## Logic

test_mech!(logic_and,"
block
  #foo = [|x|
           5
           8
          11]
block
  ix1 = #foo.x > 5
  ix2 = #foo.x <= 11
  ix3 = ix1 & ix2
  #test = stats/sum(column: #foo{ix3})", Value::F32(F32::new(19.0)));

test_mech!(logic_and_filter_inline,"
block
  #foo = [|x|
           5
           8
           11]
block
  ix = #foo.x > 5 & #foo.x <= 11
  #test = stats/sum(column: #foo{ix})", Value::F32(F32::new(19.0)));

test_mech!(logic_and_composed,"
block
  #foo = [|x|
           5
           8
           9
           11]
block
  ix = #foo.x > 5 & #foo.x <= 11 & #foo.x >= 9
  #test = stats/sum(column: #foo{ix})", Value::F32(F32::new(20.0)));

test_mech!(logic_or,"
block
  #foo = [|x|
           5
           8
           11]
block
  ix1 = #foo.x < 7
  ix2 = #foo.x > 9
  ix3 = ix1 | ix2
  #test = stats/sum(column: #foo{ix3})", Value::F32(F32::new(16.0)));

test_mech!(logic_xor,"
block
  x = [true; false; true; false]
  y = [false; true; true; false]
  ix = x xor y
  z = [1;2;3;4]
  #test = stats/sum(column: z{ix})", Value::F32(F32::new(3.0)));

test_mech!(logic_xor2,"
block
  x = [true; false; true; false]
  y = [false; true; true; false]
  ix = x ⊕ y
  z = [1;2;3;4]
  #test = stats/sum(column: z{ix})", Value::F32(F32::new(3.0)));

test_mech!(logic_xor3,"
block
  x = [true; false; true; false]
  y = [false; true; true; false]
  ix = x ⊻ y
  z = [1;2;3;4]
  #test = stats/sum(column: z{ix})", Value::F32(F32::new(3.0)));

test_mech!(logic_not,"
block
  x = [true; false; true; false]
  #y = ¬x

block
  x = [1;2;3;4]
  #test = stats/sum(column: x{#y})", Value::F32(F32::new(6.0)));

test_mech!(logic_not2,"
block
  x = [true; false; true; false]
  #y = !x

block
  x = [1;2;3;4]
  #test = stats/sum(column: x{#y})", Value::F32(F32::new(6.0)));

// ## Strings

test_mech!(string_basic,r#"
block
  #test = "Hello World""#, Value::from_str("Hello World"));

test_mech!(string_table,r#"
block
  #test = ["Hello" "World"]"#, Value::from_str("Hello"));

test_mech!(string_backslash,r#"
block
  #test = ["Hi\n"]"#, Value::from_str("Hi\\n"));

test_mech!(string_empty,r#"
block
  #test = ["" "World"]"#, Value::from_str(""));

test_mech!(string_named_attributes, r#"#test = [type: "h1" text: "An App"]"#, Value::from_str("h1"));

// ## Nesting

test_mech!(nesting_basic,r#"
block
  #app = [2 [5 7]]
  
block
  #test = #app{2}{2}"#, Value::F32(F32::new(7.0)));


test_mech!(nesting_triple,r#"
block
  #app = [1 [2 [31 3]]]
  
block
  #test = #app{2}{2}{1}"#, Value::F32(F32::new(31.0)));

test_mech!(nesting_concat,r#"
block
  ball = [1 [2 3]]
  line = [4 [5 6]]
  #out = [ball; line]
  
block
  #test = #out{2,2}{2} + #out{1,2}{1}"#, Value::F32(F32::new(8.0)));

test_mech!(nesting_math,r#"
block
  #app = [1 [2 [31 3]]]
  
block
  #test = #app{2}{2}{1} * 2"#, Value::F32(F32::new(62.0)));

test_mech!(nesting_math_select_range,r#"
block
  #app = [1 [2 [3 4 5]]]
  
block
  x = #app{2}{2}{1,:}
  #test = stats/sum(row: x)"#, Value::F32(F32::new(12.0)));

test_mech!(nesting_inline_table,r#"
block
  #robot = [x: 20 y: [x: 30 y: 50]]

block
  #test = #robot.y{:}{1} + #robot.y{:}{2}"#, Value::F32(F32::new(80.0)));


test_mech!(nesting_second_col,r#"
block
  #app2 = [1 [7 8]]
block
  #q = [_ _]
block
  #q{2} := #app2{2}
block
  #test = #q{2}{1}"#, Value::F32(F32::new(7.0)));

test_mech!(nesting_second_col2,r#"
block
  #app2 = [1 [7 8]]
block
  #q = [_ _]
block
  #q{2} := #app2{2}
block
  #test = #q{2}{2}"#, Value::F32(F32::new(8.0)));

test_mech!(nesting_chained_dot_indexing,r#"
block
  #app2 = [x: [a: 1 b: 2 c: 3] y: [x: 7 z: 8]]

block
  #test = #app2.y.z + #app2.x.b"#, Value::F32(F32::new(10.0)));

test_mech!(nesting_chained_dot_indexing_first_col,r#"
block
  #app2 = [x: [a: 1 b: 2]]

block
  #test = #app2.x.b"#, Value::F32(F32::new(2.0)));

// ## Indexing

test_mech!(indexing_global,r#"
block
  x = [true; false; true; false]
  y = [false; true; true; false]
  #y = x xor y

block
  x = [1;2;3;4]
  #test = stats/sum(column: x{#y})"#, Value::F32(F32::new(3.0)));

// ## Functions

test_mech!(function_stats_sum,r#"
block
  x = [1;2;3;4;5]
  #test = stats/sum(column: x)"#, Value::F32(F32::new(15.0)));

test_mech!(function_stats_sum_row,r#"
block
  x = [1 2 3 4 5]
  #test = stats/sum(row: x)"#, Value::F32(F32::new(15.0)));

test_mech!(function_stats_sum_row_col,r#"
block
  x = [1;2;3;4;5]
  y = stats/sum(row: x)
  #test = y{1} + y{2} + y{3} + y{4} + y{5}"#, Value::F32(F32::new(15.0)));

test_mech!(function_stats_sum_table,r#"
block
  x = [1 2 3; 4 5 6]
  #test = stats/sum(table: x)"#, Value::F32(F32::new(21.0)));

test_mech!(function_add_functions,r#"
block
  x = [1 2
       4 0
       0 7]
  #test = stats/sum(column: x{:,1}) + stats/sum(column: x{:,2})"#, Value::F32(F32::new(14.0)));

test_mech!(function_set_any,r#"
block
  x = [1; 2; 3; 4; 5]
  y = x > 4
  #test = set/any(column: y)"#, Value::Bool(true));

test_mech!(function_set_any_false,r#"
block
  x = [1; 2; 3; 4; 5]
  y = x > 5
  #test = set/any(column: y)"#, Value::Bool(false));

test_mech!(function_inline_args,r#"
block
  #test = stats/sum(row: [1 2 3 4])"#, Value::F32(F32::new(10.0)));

test_mech!(function_inline_colum_args,r#"
block
  #test = stats/sum(column: [1; 2; 3; 4])"#, Value::F32(F32::new(10.0)));

test_mech!(function_inside_anonymous_table,r#"
block
  #mech/test = ["foo", 3, stats/sum(column: 1:2)]
block
  #test = #mech/test{2} == #mech/test{3}"#, Value::Bool(true));

// ## Markdown

test_mech!(markdown_program_title, r#"# Title
  #test = 123"#, Value::F32(F32::new(123.0)));

test_mech!(markdown_no_program_title, r#"paragraph
  #test = 123"#, Value::F32(F32::new(123.0)));

test_mech!(markdown_section_title, r#"# Title

Paragraph

## Section

  #test = 123"#, Value::F32(F32::new(123.0)));

test_mech!(markdown_inline_code, r#"# Title

Paragraph including `inline code`

## Section

  #test = 123"#, Value::F32(F32::new(123.0)));

test_mech!(markdown_list, r#"# Title

Paragraph including `inline code`

## Section

- Item 1
- Item 2
- Item 3

  #test = 123"#, Value::F32(F32::new(123.0)));

test_mech!(markdown_list_inline_code, r#"# Title

Paragraph including `inline code`

## Section

- Item `some code`
- Item `some code`
- Item `some code`

  #test = 123"#, Value::F32(F32::new(123.0)));

test_mech!(markdown_code_block, r#"# Title

Paragraph including `inline code`

## Section

```
A regular code block
```

  #test = 123"#, Value::F32(F32::new(123.0)));

// ## Mechdown (Markdown extensions for Mech)

test_mech!(mechdown_inline_mech_code, r#"# Title

Paragraph including `inline mech code` is [[#test]]

## Section

  #test = 123"#, Value::F32(F32::new(123.0)));

test_mech!(mechdown_block_directives, r#"
block
  #test = 1

```mech:disabled
  #test := 2
```
"#, Value::F32(F32::new(1.0)));

test_mech!(mechdown_sub_sub_titles, r#"
# Title

block
  #x = 1
  
## Subtitle

block
  #y = 2

### Subsubtitle

block 
  #test = #x + #y"#, Value::F32(F32::new(3.0)));

// ## Comments

test_mech!(comment_line, r#"
block
  -- This is a comment
  #test = 123"#, Value::F32(F32::new(123.0)));

// ## Table split

test_mech!(table_split, r#"
block
  x = [7 8;9 6]
  #q >- x
block
  x = #q{1}{1,:}
  y = #q{2}{1,:}
  #test = stats/sum(row: [x y])"#, Value::F32(F32::new(30.0)));

test_mech!(table_split_global, r#"
block
  z = [7 8;9 6]
  q >- z
  #x = q
block
  x = #x{1}{1,:}
  y = #x{2}{1,:}
  #test = stats/sum(row: [x y])"#, Value::F32(F32::new(30.0)));

// ## Boolean values

test_mech!(boolean_anonymous_table, r#"
block
  #y = [1; 2; 3]

block
  #x = [true; false; true]
  
block
  #z = #y{#x}
  
block
  #test = #z{1} + #z{2}"#, Value::F32(F32::new(4.0)));

  test_mech!(boolean_literal_true, r#"
block
  #test = true"#, Value::Bool(true));

  test_mech!(boolean_literal_false, r#"
block
  #test = false"#, Value::Bool(false));

  test_mech!(boolean_literals_and_operator, r#"
block
  x = true
  y = false
  #test = x & y"#, Value::Bool(false));

// ## Scheduler

test_mech_txn!(scheduler_base_linear,r#"
block
  #x = [1 2 3]
block
  #y = #x + 10
block  
  #z = #y + 2
block
  #test = #z{1}"#, TXN, Value::F32(F32::new(21.0)));
  
// ## Temporal Operators

test_mech_txn!(temporal_whenever_basic_no_trigger,r#"
block
  #time/timer = [period: 16, ticks: 0]

block 
  #x = [x: 1 y: 2]
  #y = 3

block
  ~ #time/timer.ticks
  #q = #x.y
  #x.x := #x.x + #y
  
block
  #test = #x.x"#, TXN2, Value::F32(F32::new(4.0)));

test_mech_txn!(temporal_whenever_basic_with_trigger,r#"
block
  #time/timer = [period: 16, ticks: 0]

block 
  #x = [x: 1 y: 2]
  #y = 3

block
  ~ #time/timer.ticks
  #q = #x.y
  #x.x := #x.x + #y
  
block
  #test = #x.x"#, TXN3, Value::F32(F32::new(7.0)));

test_mech_txn!(temporal_whenever_blocks,r#"
block
  #time/timer = [period: 1000, ticks: 0]
  #balls = [|x y vx vy|
             1.0 1.0 1.0  1.0
             50.0 80.0 2.0  10.0]
  #gravity = 1.0

block
  ~ #time/timer.ticks
  #balls.x := #balls.x + #balls.vx
  #balls.y := #balls.y + #balls.vy
  #balls.vy := #balls.vy + #gravity
  
Keep the balls within the boundary height
  ~ #balls.y
  iy = #balls.y > 100.0
  #balls.y{iy} := 100.0
  #balls.vy{iy} := #balls.vy * -0.80
block  
  #test = #balls.y{2} + #balls.vy{2}"#, TXN4, Value::F32(F32::new(81.8)));
  