extern crate mech_syntax;

use mech_syntax::parser;

macro_rules! test_parser {
  ($func:ident, $input:tt, $($expected_err_loc:expr),*) => (
    #[test]
    fn $func() {
        let err_locations = vec![$($expected_err_loc),*];
        let parse_result = parser::parse($input);
        if (err_locations.is_empty()) {
          assert!(parse_result.is_ok());
          return;
        }
        assert!(parse_result.is_err());
    }
  )
}

test_parser!(name, r#"
block
  prog
"#, (1, 3));
