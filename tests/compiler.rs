#[macro_use]
extern crate mech_syntax;
extern crate mech_core;

use mech_syntax::parser::{Parser, Node};
use mech_syntax::compiler::Compiler;
use mech_core::{Hasher, Core, Index, Value, make_quantity};

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

macro_rules! test_mech {
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
        None => assert_eq!(0,1),
      }
    }
  )
}

compile_string!(empty, "");

// ## Constant

compile_string!(constant_digit, "1");

test_mech!(constant_empty, "
block
  x = [1 2
       4 _
       _ 7]
  #test = stat/sum(column: x{:,1})",Value::from_i64(5));

test_mech!(constant_inline_empty, "#test = [first: 123, second: _, third: 456]",Value::from_i64(123));

// ## Table

compile_string!(table, "#table");

compile_string!(table_define, "#table = [x y z]");

compile_string!(table_define_data, "#table =  [x y z | 1 2 3]");

compile_string!(table_define_data_math, "#table = [x      y          z|
                                                   1 * 2, 4 + 7 * 9, 3]");

test_mech!(table_define_program, "# A Working Program

## Section Two

  #test = 9", Value::from_i64(9));

// ## Select

test_mech!(select_table,"  
block
  #x = 500
block
  #test = #x", Value::from_i64(500));

test_mech!(select_table_reverse_ordering,"  
block
  #test = #x
block
  #x = 500", Value::from_i64(500));

// ## Math

test_mech!(math_constant,"#test = 10", Value::from_i64(10));

test_mech!(math_add,"#test = 1 + 1", Value::from_i64(2));

test_mech!(math_multiply,"#test = 2 * 2", Value::from_i64(4));

test_mech!(math_divide,"#test = 4 / 2", Value::Number(make_quantity(20000,-4,0)));

test_mech!(math_two_terms,"#test = 1 + 2 * 9", Value::from_i64(19));

test_mech!(math_constant_collision,"#test = 10000 + 1", Value::from_i64(10001));

test_mech!(math_multiple_variable_graph,"block
  a = z * 5
  #test = d * z + a
  d = 9 * z
  z = 5", Value::from_i64(250));

test_mech!(math_multiple_variable_graph_new_ordering,"block
  #test = d * z + a
  a = z * 5
  z = 5
  d = 9 * z", Value::from_i64(250));

test_mech!(math_on_whole_table,"
block
  #x = 500
block
  #test = #x + 5", Value::from_i64(505));

test_mech!(select_column_by_id,"  
block
  #ball = [x: 56 y: 2 vx: 3 vy: 4]
block
  #test = #ball.x", Value::from_i64(56));

test_mech!(math_multiple_rows_select,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = #ball.x + #ball.y * #ball.vx", Value::from_i64(177));

test_mech!(math_const_and_select,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = 9 + #ball.x", Value::from_i64(24));

test_mech!(math_select_and_const,"
block
  #ball = [x: 15 y: 9 vx: 18 vy: 0]
block
  #test = #ball.x + 9", Value::from_i64(24));

test_mech!(partial_bouncing_ball,"# Bouncing Balls
Define the environment
  #ball = [x: 15 y: 9 vx: 18 vy: 9]
  #system/timer = [resolution: 1000]
  #gravity = 10

Now update the block positions
  x = #ball.x + #ball.vx
  y = #ball.y + #ball.vy
  dt = #system/timer.resolution
  #test = x + y * dt", Value::from_i64(18033));

test_mech!(math_add_columns,"
block
  #ball = [|x y|
            1 2
            3 4
            5 6]
block
  #test = #ball.x + #ball.y", Value::from_i64(3));

test_mech!(math_add_matrices,"
block
  x = [1 2 3
       4 5 6
       7 8 9]
  y = [10 11 12
       13 14 15
       16 17 18]
  #test = x + y", Value::from_i64(11));

test_mech!(math_scalar_plus_vector,"
block
  x = 3:6
  #test = 5 + x", Value::from_i64(8));

test_mech!(math_vector_plus_scalar,"
block
  x = 3:6
  #test = x + 5", Value::from_i64(8));

test_mech!(math_negation_double_negative,"
block
  y = -13
  #test = -y", Value::from_i64(13));

test_mech!(math_parenthetical_expression_constants,"
block
  #test = (1 + 2) * 3", Value::from_i64(9));

// ## Ranges

test_mech!(range_basic,r#"
block
  #test = stat/sum(column: #range)
block
  #range = 5 : 14"#, Value::from_i64(95));

// ## Subscripts

test_mech!(subscript_scalar_math,"
block
  x = 3:6
  y = 10:12
  #test = x{1,1} + y{3,1}", Value::from_i64(15));

test_mech!(subscript_scan,"
block
  x = 10:20
  z = 3:5
  #test = x{z, :}", Value::from_i64(12));

test_mech!(subscript_logical_greater,"
block
  x = 10:20
  z = x > 15
  #test = x{z, :}", Value::from_i64(16));

test_mech!(subscript_logical_less,"
block
  x = 10:20
  z = x < 15
  #test = x{z, :}", Value::from_i64(10));

// ## Set

test_mech!(set_column_simple,"
block
  #test.x{1} := 77

block
  #test = [|x|
            9]", Value::from_i64(77));

test_mech!(set_column_logical,"
block
  ix = x > 0
  x = #test.x
  #test.x{ix} := 3

block
  #test = [|x y z|
            1 2 3
            4 5 6
            7 8 9]", Value::from_i64(3));

test_mech!(set_second_column_logical,"
block
  #test = #ball.y

block
  ix = x > 0
  x = #ball.y
  #ball.y{ix} := 3

block
  #ball = [|x y z|
            1 2 3
            4 5 6
            7 8 9]", Value::from_i64(3));

test_mech!(set_second_omit_row_subscript,"
block
  #ball = [x: 15 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15 tick: 0]
  #gravity = 2

block
  ~ #system/timer.tick
  #ball.y := #ball.vy + #gravity

block
  #test = #ball.y", Value::from_i64(11));

test_mech!(set_rhs_math_filters_logic,"
block
  #ball = [|x y vx vy|
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

test_mech!(set_implicit_logic,"
block
  #ball = [|x y vx vy|
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

// ## Concat

test_mech!(concat_horzcat_data,"
block
  x = 1:10
  y = 11:20
  #z = [x y]
  
block
  #test = #z{1,1} + #z{1,2} + #z{2,1} + #z{1,1}", Value::from_i64(15));

// ## Append

test_mech!(append_row_inline,"
block
  ix = #foo.x > 50
  #test = #foo{ix, :}

block
  ~ #z.x
  y = #z
  #foo += [x: 100 y: 110 z: 120]

block
  x = #ball.y
  #z = [x: 123 y: 456]
  #foo = [|x y z|
           5 6 7
           8 9 10
           11 12 13]

block
  #ball = [|x y z|
            1 2 3]", Value::from_i64(100));

// ## Logic

test_mech!(logic_and,"
block
  ix1 = #foo.x > 5
  ix2 = #foo.x < 11
  ix3 = ix1 & ix2
  #test = #foo{ix3, 1}

block
  #foo = [|x y z|
           5 6 7
           8 9 10
           11 12 13]", Value::from_i64(8));

test_mech!(logic_and_filter_inline,"
block
  ix = #foo.x > 5 & #foo.x < 11
  #test = #foo{ix, 1}

block
  #foo = [|x y z|
           5 6 7
           8 9 10
           11 12 13]", Value::from_i64(8));

test_mech!(logic_and_composed,"
block
  ix = #foo.x > 5 & #foo.x < 11 & #foo.y > 9
  #test = #foo{ix, 1}

block
  #foo = [|x y z|
           5 6 7
           8 9 10
           9 10 11
           11 12 13]", Value::from_i64(9));

test_mech!(logic_or,"
block
  ix1 = #foo.x < 7
  ix2 = #foo.x > 9
  ix3 = ix1 | ix2
  #test = #foo{ix3, 1}

block
  #foo = [|x y z|
           5 6 7
           8 9 10
           11 12 13]", Value::from_i64(5));

// ## Change scan

test_mech!(change_scan_simple,"block
  #system/timer = [tick: 0]

block
  ~ #system/timer.tick
  #test = 3", Value::from_i64(3));

// ## Full programs

test_mech!(program_Clock,r#"# Clock

Create a timer that ticks every second. This is the time source.
  #system/timer = [resolution: 1000, tick: 0, hours: 2, minutes: 32, seconds: 47]

Set up a clock hands table. Degrees is the deflection from noon.
x and y are the coordinates of the end point of the clock hand.
  #clock-hands = [|degrees length stroke    x y |
                   0       30     "023963" 0 0
                   0       40     "023963" 0 0
                   0       40     "ce0b46" 0 0 ]

## Update the clock

Calculate clock hand angles every time the clock ticks.
  ~ #system/timer.tick 
  time = [#system/timer.hours; #system/timer.minutes; #system/timer.seconds]
  multiplier = [30; 6; 6]
  #clock-hands.degrees := multiplier * time
  
Calculate x and y endpoints
  angle = #clock-hands.degrees
  #clock-hands.x := 50 + (30 * math/sin(degrees: angle))
  #clock-hands.y := 50 - (30 * math/cos(degrees: angle))
  
test
  x = stat/sum(column: #clock-hands{:,1})
  y = stat/sum(column: #clock-hands{:,4})
  z = stat/sum(column: #clock-hands{:,5})
  #test = x + y + z"#, Value::Number(make_quantity(83250606066446,-11,0)));

test_mech!(program_bouncing_balls,"# Bouncing Balls

Define the environment
  #html/event/click = [|x y|]
  #ball = [x: 50 y: 9 vx: 40 vy: 9]
  #system/timer = [resolution: 15, tick: 0]
  #gravity = 2
  #boundary = [x: 60 y: 60]

## Update condition

Now update the block positions
  ~ #system/timer.tick
  #ball.x := #ball.x + #ball.vx
  #ball.y := #ball.y + #ball.vy
  #ball.vy := #ball.vy + #gravity

## Boundary Condition

Keep the balls within the y boundary
  ~ #ball.y
  iy = #ball.y > #boundary.y
  #ball.y{iy} := #boundary.y
  #ball.vy{iy} := -#ball.vy * 80 / 100

Keep the balls within the x boundary
  ~ #ball.x
  ix = #ball.x > #boundary.x
  ixx = #ball.x < 0
  #ball.x{ix} := #boundary.x
  #ball.x{ixx} := 0
  #ball.vx{ix | ixx} := -#ball.vx * 80 / 100

## Create More Balls

Create ball on click
  ~ #html/event/click.x
  #ball += [x: 10 y: 10 vx: 40 vy: 0]

test
  x = #ball.x + #ball.y
  #test = stat/sum(column: x)", Value::Number(make_quantity(138,0,0)));

// ## Strings

test_mech!(string_basic,r#"
block
  #test = "Hello World""#, Value::from_str("Hello World"));

test_mech!(string_table,r#"
block
  #test = ["Hello" "World"]"#, Value::from_str("Hello"));

test_mech!(string_empty,r#"
block
  #test = ["" "World"]"#, Value::from_str(""));

test_mech!(string_named_attributes, r#"#test = [type: "h1" text: "An App"]"#, Value::from_str("h1"));

// ## Nesting

test_mech!(nesting_basic,r#"
block
  x = [#app{1,2}{1,1}]
  y = [#app{1,2}{2,1}]
  #test = x + y

block
  div = "div"
  h1 = "h1"
  container = [|type text| 
                123   "A Mech Webpage"
                456   "Hello World"]
  #app = [|direction contains| 
           "column"  [container]
           "row"     [container]]"#, Value::from_u64(579));


test_mech!(nesting_triple,r#"
block
  #test = [#app{2,2}{1,2}{1,1}]

block
  x = 314
  container = [|type text| 
                123   [x]]
  #app = [|direction contains| 
           "column"  [container]
           "row"     [container]]"#, Value::from_u64(314));

test_mech!(nesting_math,r#"
block
  #test = #app{2,2}{1,2}{1,1} * 10

block
  x = 314
  container = [|type text| 
                123   [x]]
  #app = [|direction contains| 
           "column"  [container]
           "row"     [container]]"#, Value::from_u64(3140));

test_mech!(nesting_math_select_range,r#"
block
  x = #app{2,2}{1,2}{:,1} * 10
  y = x{2,1}
  z = x{3,1}
  #test = y + z

block
  x = 1:10
  container = [|type text| 
                123   [x]]
  #app = [|direction contains| 
           "column"  [container]
           "row"     [container]]"#, Value::from_u64(50));

// ## Functions

test_mech!(function_math_sin_degrees,r#"
block
  #test = math/sin(degrees: 90)"#, Value::Number(make_quantity(100000000000000,-14,0)));

test_mech!(function_math_sin_degrees_180,r#"
block
  #test = math/sin(degrees: 180)"#, Value::Number(make_quantity(0,0,0)));

test_mech!(function_math_sin_210,r#"
block
  #test = math/sin(degrees: 210)"#, Value::Number(make_quantity(-50000000000000,-14,0)));

test_mech!(function_math_cos_210,r#"
block
  #test = math/cos(degrees: 210)"#, Value::Number(make_quantity(-86602540378443,-14,0)));

test_mech!(function_math_cos_degrees,r#"
block
  #test = math/cos(degrees: 0)"#, Value::Number(make_quantity(100000000000000,-14,0)));

test_mech!(function_stat_sum,r#"
block
  x = [1;2;3;4;5]
  #test = stat/sum(column: x)"#, Value::Number(make_quantity(15,0,0)));

test_mech!(function_add_functions,r#"
block
  x = [1 2
       4 _
       _ 7]
  #test = stat/sum(column: x{:,1}) + stat/sum(column: x{:,2})"#, Value::from_i64(14));

// ## Errors

test_mech!(error_duplicate_alias, r#"
block
  #test = 5

block
  x = 1
  x = 3
  #test := 7"#, Value::from_i64(5));

// ## Markdown

test_mech!(markdown_program_title, r#"# Title
  #test = 123"#, Value::from_i64(123));

test_mech!(markdown_no_program_title, r#"paragraph
  #test = 123"#, Value::from_i64(123));

test_mech!(markdown_section_title, r#"# Title

Paragraph

## Section

  #test = 123"#, Value::from_i64(123));

test_mech!(markdown_inline_code, r#"# Title

Paragraph including `inline code`

## Section

  #test = 123"#, Value::from_i64(123));

test_mech!(markdown_list, r#"# Title

Paragraph including `inline code`

## Section

- Item 1
- Item 2
- Item 3

  #test = 123"#, Value::from_i64(123));

test_mech!(markdown_list_inline_code, r#"# Title

Paragraph including `inline code`

## Section

- Item `some code`
- Item `some code`
- Item `some code`

  #test = 123"#, Value::from_i64(123));

test_mech!(markdown_code_block, r#"# Title

Paragraph including `inline code`

## Section

```
A regular code block
```

  #test = 123"#, Value::from_i64(123));

// ## Mechdown (Markdown extensions for Mech)

test_mech!(mechdown_inline_mech_code, r#"# Title

Paragraph including `inline mech code` is [[#test]]

## Section

  #test = 123"#, Value::from_i64(123));
