#[macro_use]
extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::lexer::Lexer;
use mech_syntax::parser::{Parser, ParseStatus, Node};
use mech_syntax::compiler::Compiler;
use mech_core::{Hasher, Core, Index, Value};

macro_rules! compile_string {
  ($func:ident, $test:tt) => (
    #[test]
    fn $func() {
      let mut compiler = Compiler::new();
      let input = String::from($test);
      compiler.compile_string(input);
      assert_eq!(compiler.errors.is_empty(), true);
    }
  )
}

macro_rules! test_math {
  ($func:ident, $input:tt, $test:expr) => (
    #[test]
    fn $func() {
      let mut compiler = Compiler::new();
      let mut core = Core::new(10, 10);
      let input = String::from($input);
      compiler.compile_string(input);
      core.register_blocks(compiler.blocks);
      core.step();
      let table = Hasher::hash_str("test");
      let row = Index::Index(1);
      let column = Index::Index(1);
      let test: Value = $test;
      let actual = core.index(table, &row, &column);
      match actual {
        Some(value) => {
          assert_eq!(*value, test);
        },
        _ => (),
      }
      
    }
  )
}

compile_string!(empty, "");

// ## Constant

compile_string!(constant_digit, "1");

// ## Table

compile_string!(table, "#table");

compile_string!(table_define, "#table = [x y z]");

compile_string!(table_define_data, "#table = [x y z
                                              1 2 3]");

compile_string!(table_define_data_math, "#table = [x      y          z
                                                   1 * 2, 4 + 7 * 9, 3]");

test_math!(table_define_program, "# A Working Program

## Section Two

  #test = 9", Value::from_i64(9));

// ## Select

test_math!(select_table,"  
block
  #x = 500
block
  #test = #x", Value::from_i64(500));

test_math!(select_table_reverse_ordering,"  
block
  #test = #x
block
  #x = 500", Value::from_i64(500));

// ## Math

test_math!(math_constant,"#test = 10", Value::from_i64(10));

test_math!(math_add,"#test = 1 + 1", Value::from_i64(2));

test_math!(math_multiply,"#test = 2 * 2", Value::from_i64(4));

test_math!(math_divide,"#test = 4 / 2", Value::from_i64(2));

test_math!(math_two_terms,"#test = 1 + 2 * 9", Value::from_i64(19));

test_math!(math_multiple_variable_graph,"block
  a = z * 5
  #test = d * z + a
  d = 9 * z
  z = 5", Value::from_i64(250));

test_math!(math_multiple_variable_graph_new_ordering,"block
  #test = d * z + a
  a = z * 5
  z = 5
  d = 9 * z", Value::from_i64(250));

test_math!(math_on_whole_table,"
block
  #x = 500
block
  #test = #x + 5", Value::from_i64(505));

test_math!(select_column_by_id,"  
block
  #ball = [x: 56 y: 2 vx: 3 vy: 4]
block
  #test = #ball.x", Value::from_i64(56));

test_math!(math_multiple_rows_select,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = #ball.x + #ball.y * #ball.vx", Value::from_i64(177));

test_math!(math_const_and_select,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = 9 + #ball.x", Value::from_i64(24));

test_math!(math_select_and_const,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = #ball.x + 9", Value::from_i64(24));

test_math!(partial_bouncing_ball,"# Bouncing Balls
Define the environment
  #ball = [x: 15 y: 9 vx: 18 vy: 9]
  #system/timer = [resolution: 1000]
  #gravity = 10

Now update the block positions
  x = #ball.x + #ball.vx
  y = #ball.y + #ball.vy
  dt = #system/timer.resolution
  #test = x + y * dt", Value::from_i64(18033));

test_math!(math_add_columns,"
block
  #ball = [x y
           1 2
           3 4
           5 6]
block
  #test = #ball.x + #ball.y", Value::from_i64(3));

test_math!(math_add_matrices,"
block
  x = [1 2 3
       4 5 6
       7 8 9]
  y = [10 11 12
       13 14 15
       16 17 18]
  #test = x + y", Value::from_i64(11));

test_math!(math_scalar_plus_vector,"
block
  x = 3:6
  #test = 5 + x", Value::from_i64(8));

test_math!(math_vector_plus_scalar,"
block
  x = 3:6
  #test = x + 5", Value::from_i64(8));

test_math!(math_negation_double_negative,"
block
  y = -13
  #test = -y", Value::from_i64(13));

test_math!(math_parenthetical_expression_constants,"
block
  #test = (1 + 2) * 3", Value::from_i64(9));

// ## Ranges

test_math!(range_basic,"#test = 5 : 14", Value::from_i64(5));

// ## Subscripts

test_math!(subscript_scalar_math,"
block
  x = 3:6
  y = 10:12
  #test = x{1,1} + y{3,1}", Value::from_i64(15));

test_math!(subscript_scan,"
block
  x = 10:20
  z = 3:5
  #test = x{z, :}", Value::from_i64(12));

test_math!(subscript_logical_greater,"
  block
  x = 10:20
  z = x > 15
  #test = x{z, :}", Value::from_i64(16));

test_math!(subscript_logical_less,"
  block
  x = 10:20
  z = x < 15
  #test = x{z, :}", Value::from_i64(10));

// ## Set

test_math!(set_column_logical,"
block
  ix = x > 0
  x = #test.x
  #test.x{ix} := 3

block
  #test = [x y z
           1 2 3
           4 5 6
           7 8 9]", Value::from_i64(3));

test_math!(set_second_column_logical,"
block
  #test = #ball.y

block
  ix = x > 0
  x = #ball.y
  #ball.y{ix} := 3

block
  #ball = [x y z
           1 2 3
           4 5 6
           7 8 9]", Value::from_i64(3));

test_math!(set_second_omit_row_subscript,"
block
  #ball = [x: 15 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15 tick: 0]
  #gravity = 2

block
  #system/timer.tick
  #ball.y := #ball.vy + #gravity

block
  #test = #ball.y", Value::from_i64(11));

test_math!(set_rhs_math_filters_logic,"
block
  #ball = [x y vx vy
           1 2 3 4
           5 6 7 8
           9 10 11 12]
  #system/timer = [resolution: 15 tick: 0]
  #gravity = 2

block
  ix = #ball.vy > 10
  iy = #ball.vy < 5
  ixx = ix | iy
  #ball.y{ixx} := #ball.vy * 9099

block
  #test = #ball{1,2} + #ball{3,2}", Value::from_i64(145584));

test_math!(set_implicit_logic,"
block
  #ball = [x y vx vy
           1 2 3 4
           5 6 7 8
           9 10 11 12]
  #system/timer = [resolution: 15 tick: 0]
  #gravity = 2

block
  ix = #ball.vy > 10
  iy = #ball.vy < 5
  #ball.y{ix | iy} := #ball.vy * 9099

block
  #test = #ball{1,2} + #ball{3,2}", Value::from_i64(145584));

// ## Append

test_math!(append_row_inline,"
block
  ix = #foo.x > 50
  #test = #foo{ix, :}

block
  y = #z
  #foo += [x: 100 y: 110 z: 120]

block
  x = #ball.y
  #z = [x: 123 y: 456]
  #foo = [x y z
           5 6 7
           8 9 10
           11 12 13]

block
  #ball = [x y z
           1 2 3]", Value::from_i64(100));

// ## Logic

test_math!(logic_and,"
block
  ix1 = #foo.x > 5
  ix2 = #foo.x < 11
  ix3 = ix1 & ix2
  #test = #foo{ix3, 1}

block
  #foo = [x y z
           5 6 7
           8 9 10
           11 12 13]", Value::from_i64(8));

test_math!(logic_or,"
block
  ix1 = #foo.x < 7
  ix2 = #foo.x > 9
  ix3 = ix1 | ix2
  #test = #foo{ix3, 1}

block
  #foo = [x y z
           5 6 7
           8 9 10
           11 12 13]", Value::from_i64(5));

// ## Change scan

test_math!(change_scan_simple,"
block
  #ball = [x y z 
           1 2 3
           4 5 6
           7 8 9]
  #system/timer = [resolution: 15 tick: 0]
  #gravity = 2

block
  ~ #system/timer.tick
  ix = #ball.x > 5
  ixx = #ball.x < 2
  #ball.x{ix | ixx} := #ball.x + #ball.z
  
block
  #test = #ball{1,1} + #ball{2,1} + #ball{3,1}", Value::from_i64(24));

// ## Full programs

test_math!(program_bouncing_balls,"# Bouncing Balls

Define the environment
  #html/event/click = [x: 0 y: 0]
  #ball = [x: 50 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = 60

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

## Boundary Condition

Keep the balls within the y boundary
  ~ #ball.y
  iy = #ball.y > #boundary
  #ball.y{iy} := #boundary
  #ball.vy{iy} := -#ball.vy * 80 / 100

Keep the balls within the x boundary
  ~ #ball.x
  ix = #ball.x > #boundary
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary
  #ball.x{ixx} := 0
  #ball.vx{ix | ixx} := -#ball.vx * 80 / 100

## Create More Balls

Create ball on click
  ~ #html/event/click.x
  #ball += [x: 10 y: 10 vx: 40 vy: 0]
  
block
  #test = #ball{1,1} + #ball{1,3} + #ball{2,1} + #ball{2,3}", Value::from_i64(118));

// ## Strings

test_math!(string_basic,"
block
  #ball = \"Hello World\"", Value::from_str("Hello World"));

test_math!(string_table,"
block
  #x = [\"Hello\"  \"World\"]", Value::from_str("Hello"));