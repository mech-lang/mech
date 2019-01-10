#[macro_use]
extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::lexer::Lexer;
use mech_syntax::parser::{Parser, ParseStatus, Node};
use mech_syntax::compiler::Compiler;
use mech_core::{Hasher, Core, Index};

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
  ($func:ident, $input:tt, $test:tt) => (
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
      let test = $test;
      assert_eq!(core.index(table, &row, &column).unwrap().as_u64().unwrap(), test);
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

  #test = 9", 9);

// ## Select

test_math!(select_table,"  
block
  #x = 500
block
  #test = #x", 500);

test_math!(select_table_reverse_ordering,"  
block
  #test = #x
block
  #x = 500", 500);

// ## Math

test_math!(math_constant,"#test = 10", 10);

test_math!(math_add,"#test = 1 + 1", 2);

test_math!(math_multiply,"#test = 2 * 2", 4);

test_math!(math_divide,"#test = 4 / 2", 2);

test_math!(math_two_terms,"#test = 1 + 2 * 9", 19);

test_math!(math_multiple_variable_graph,"block
  a = z * 5
  #test = d * z + a
  d = 9 * z
  z = 5", 250);

test_math!(math_multiple_variable_graph_new_ordering,"block
  #test = d * z + a
  a = z * 5
  z = 5
  d = 9 * z", 250);

test_math!(math_on_whole_table,"
block
  #x = 500
block
  #test = #x + 5", 505);

test_math!(select_column_by_id,"  
block
  #ball = [x: 56 y: 2 vx: 3 vy: 4]
block
  #test = #ball.x", 56);

test_math!(math_multiple_rows_select,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = #ball.x + #ball.y * #ball.vx", 177);

test_math!(math_const_and_select,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = 9 + #ball.x", 24);

test_math!(math_select_and_const,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = #ball.x + 9", 24);

test_math!(partial_bouncing_ball,"# Bouncing Balls
Define the environment
  #ball = [x: 15 y: 9 vx: 18 vy: 9]
  #system/timer = [resolution: 1000]
  #gravity = 10

Now update the block positions
  x = #ball.x + #ball.vx
  y = #ball.y + #ball.vy
  dt = #system/timer.resolution
  #test = x + y * dt", 18033);

test_math!(math_add_columns,"
block
  #ball = [x y
           1 2
           3 4
           5 6]
block
  #test = #ball.x + #ball.y", 3);

test_math!(math_add_matrices,"
block
  x = [1 2 3
       4 5 6
       7 8 9]
  y = [10 11 12
       13 14 15
       16 17 18]
  #test = x + y", 11);

test_math!(math_scalar_plus_vector,"
block
  x = 3:6
  #test = 5 + x", 8);

test_math!(math_vector_plus_scalar,"
block
  x = 3:6
  #test = x + 5", 8);

// ## Ranges

test_math!(range_basic,"#test = 5 : 14", 5);

// ## Subscripts

test_math!(subscript_scalar_math,"
block
  x = 3:6
  y = 10:12
  #test = x{1,1} + y{3,1}", 15);

test_math!(subscript_scan,"
block
  x = 10:20
  z = 3:5
  #test = x{z, :}", 12);

test_math!(subscript_logical_greater,"
  block
  x = 10:20
  z = x > 15
  #test = x{z, :}", 16);

test_math!(subscript_logical_less,"
  block
  x = 10:20
  z = x < 15
  #test = x{z, :}", 10);

// Set

test_math!(set_column_logical,"
block
  ix = x > 0
  x = #test.x
  #test.x{ix} := 3

block
  #test = [x y z
           1 2 3
           4 5 6
           7 8 9]", 3);

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
           7 8 9]", 3);

test_math!(set_second_omit_row_subscript,"
block
  #ball = [x: 15 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15 tick: 0]
  #gravity = 2

block
  #system/timer.tick
  #ball.y := #ball.vy + #gravity

block
  #test = #ball.y", 11);

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
  #test = #ball{1,2} + #ball{3,2}", 145584);

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
  #test = #ball{1,2} + #ball{3,2}", 145584);

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
           1 2 3]", 100);

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
           11 12 13]", 8);

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
           11 12 13]", 5);