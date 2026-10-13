"""Independent field/enum canonical integration fixtures; mutex execution."""
import unittest
from normalizer import normalize


class RecordIntegrationFixtures(unittest.TestCase):
    def count(self, source, language, units):
        row = normalize(source, language)
        self.assertEqual(row['normalized_loc'], units, row)
        self.assertEqual(row['diagnostics'], [], row)
        return row

    def test_rust_named_fields_packed_and_expanded(self):
        for source in ('struct Body{x:f64,y:f64,z:f64,vx:f64,vy:f64,vz:f64,mass:f64}',
                       'struct Body {\nx:f64,\ny:f64,\nz:f64,\nvx:f64,\nvy:f64,\nvz:f64,\nmass:f64\n}'):
            self.count(source, 'rust', 9)

    def test_generic_type_commas_and_tuple_positional_types_do_not_split(self):
        self.count("struct Tree<'a>{left:Option<&'a Tree<'a>>,right:Result<Node,Error>}", 'rust', 4)
        self.count('struct Vec3(pub f64,pub f64,pub f64);', 'rust', 1)

    def test_named_enum_variants_packed_and_expanded(self):
        for source in ('pub enum c_void{__variant1,__variant2}',
                       'pub enum c_void {\n__variant1,\n__variant2\n}'):
            self.count(source, 'rust', 4)
        self.count("enum Item{Freq(usize),Occ(&'static str)}", 'rust', 4)

    def test_c_cpp_and_java_enum_constants_share_units(self):
        for language in ('c','cpp','java'):
            for source in ('enum E{A,B,C}', 'enum E{\nA,\nB,\nC\n}'):
                self.count(source, language, 5)

    def test_typedef_enum_primary_alias_stays_in_header(self):
        self.count('typedef enum{A,B}Alias;', 'c', 4)

    def test_literal_constructors_retain_their_data_rows(self):
        self.count('let value=Body{x:1,y:2};', 'rust', 1)
        self.count('let values=[\nBody{x:1,y:2},\nBody{x:3,y:4}\n];', 'rust', 4)

    def test_complex_enum_member_and_nested_record_body_stay_reviewed(self):
        for source, language in (('enum E{A{value:u32},B}', 'rust'),
                                 ('enum E{A(){void f(){}},B}', 'java')):
            row = normalize(source, language)
            self.assertTrue(any(d['kind']=='declaration-field-review' for d in row['diagnostics']), row)
            self.assertFalse(any(e['kind']=='declaration-field' for e in row['canonical_events']), row)


if __name__ == '__main__':
    from audit import timing_boundary
    with timing_boundary():
        unittest.main()
