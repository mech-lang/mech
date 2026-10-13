"""Unexecuted authoritative fixture proposals for keyword source units.

Run only in the reviewed common-mutex job. Expected counts follow the approved
header/open/body/written-close rule and declaration semantics, not an execution
receipt or a mirror of classifier internals. No benchmark/report I/O is used.
"""
import unittest

from foreign_keyword import classify_scopes,mask_keyword,normalize


class KeywordProposalFixtures(unittest.TestCase):
    def assert_units(self,source,language,expected):
        metric=normalize(source,language)
        self.assertEqual(metric['diagnostics'],[],metric['diagnostics'])
        self.assertEqual(metric['normalized_loc'],expected)
        self.assertTrue(metric['proposal_only'])
        self.assertEqual(metric['proposal_status'],'unhooked-no-admission')
        return metric

    def assert_gate(self,source,language,kind):
        metric=normalize(source,language)
        self.assertIn(kind,{item['kind'] for item in metric['diagnostics']})
        return metric

    def test_ada_grouped_locals_packed_expanded(self):
        self.assert_units('procedure P is A,B: Integer; begin A:=1; B:=2; end P;','ada',6)
        self.assert_units('procedure P is\nA: Integer;\nB: Integer;\nbegin\nA:=1;\nB:=2;\nend P;','ada',6)

    def test_ada_multiline_formal_header_is_one(self):
        self.assert_units('procedure P\n(A: Integer;\n B: Integer := 1) is\nbegin\nnull;\nend P;','ada',3)

    def test_ada_formal_renames_and_forward_declarations(self):
        self.assert_units('procedure Put (X: Real; A: Integer := 0; B: Integer := 9) renames Other.Put;','ada',1)
        self.assert_units('procedure Put\n(X: Real;\n A: Integer := 0;\n B: Integer := 9)\nrenames Other.Put;','ada',1)
        self.assert_units('function F(X: Integer; Y: Integer) return Integer;','ada',1)

    def test_ada_written_modifiers_are_owned_by_one_function_header(self):
        self.assert_units('overriding procedure Initialize(X: T) is begin Call(X); end Initialize;','ada',3)
        self.assert_units('overriding\nprocedure Initialize(X: T) is\nbegin\nCall(X);\nend Initialize;','ada',3)
        self.assert_units('not overriding procedure Initialize(X: T);','ada',1)
        self.assert_units('not\noverriding\nprocedure Initialize(X: T);','ada',1)
        self.assert_units('package body P.Q is\noverriding\nprocedure Initialize(X: T) is begin Call(X); end Initialize;\nend P.Q;','ada',5)
        self.assert_units('package P is private overriding procedure Initialize(X: T); end P;','ada',3)
        self.assert_units('package P is\nprivate\noverriding\nprocedure Initialize(X: T);\nend P;','ada',3)
        self.assert_gate('private\nX: Integer;','ada','foreign-ada-private-section-review')

    def test_ada_written_pragma_group_settings_are_structural(self):
        self.assert_units('pragma Import(C,Print,Symbol);','ada',1)
        self.assert_units('pragma Import(\n C,\n Print,\n Symbol);','ada',1)
        self.assert_units('procedure P is pragma Inline(F,G); begin Call; end P;','ada',4)
        self.assert_units('procedure P is\npragma Inline(\n F,\n G);\nbegin\nCall;\nend P;','ada',4)
        self.assert_gate('pragma Import(\nC,Print;','ada','foreign-ada-pragma-owner-review')

    def test_ada_based_literal_owns_only_valid_complete_digits(self):
        for literal in ('16#100#','16#F_F#','2#101.01#E+2','10#1_000#'):
            source='pragma Storage_Size('+literal+');'
            metric=self.assert_units(source,'ada',1)
            self.assertEqual(metric['based_literal_evidence'][0]['spelling'],literal)
            masked=mask_keyword(source,'ada')['cleaned']
            self.assertEqual(len(masked),len(source))
        self.assert_units('X:=16#FF#;','ada',1)
        for literal in ('2#2#','17#10#','16#10','16#1__0#'):
            self.assert_gate('X:='+literal+';','ada','foreign-ada-based-literal-review')
        metric=self.assert_units('-- 16#100#\nX:=\"16#FF#\";','ada',1)
        self.assertEqual(metric['based_literal_evidence'],[])

    def test_ada_named_qualified_and_loop_close_suffix_is_structural(self):
        self.assert_units('package P.Q is A: Integer; end P.Q;','ada',3)
        self.assert_units('package P.Q is\nA: Integer;\nend\nP.Q;','ada',3)
        self.assert_units('procedure P is begin Work: loop Call; end loop Work; end P;','ada',5)
        self.assert_units('procedure P is\nbegin\nWork:\nloop\nCall;\nend loop\nWork;\nend P;','ada',5)
        self.assert_gate('package P.Q is A: Integer; end P.Other;','ada','foreign-ada-close-name-review')
        self.assert_gate('Work: loop Call; end loop Other;','ada','foreign-ada-loop-name-review')

    def test_ada_records_fields_and_named_associations(self):
        self.assert_units('type Node is record Left,Right: Pointer; end record;','ada',4)
        self.assert_units('type Node is record\nLeft: Pointer;\nRight: Pointer;\nend record;','ada',4)
        self.assert_units('Call(Item=>A,Width=>1);','ada',1)

    def test_ada_record_keyword_modifiers_and_context_imports_are_owned(self):
        self.assert_units('type Node is limited record Left,Right: Pointer; end record;','ada',4)
        self.assert_units('type Node is\nlimited\nrecord\nLeft: Pointer;\nRight: Pointer;\nend record;','ada',4)
        self.assert_units('private with Ada.Finalization, Runtime.Pools;','ada',1)
        self.assert_units('private\nwith Ada.Finalization,\nRuntime.Pools;','ada',1)
        self.assert_gate('private with malformed(x);','ada','foreign-ada-private-section-review')

    def test_ada_derived_record_parent_is_header_structure_not_imputed_fields(self):
        self.assert_units('type Node is new Runtime.Parent with record Field: Pointer; end record;','ada',3)
        self.assert_units('type Node is\nnew Runtime.Parent with\nrecord\nField: Pointer;\nend record;','ada',3)
        self.assert_units('type Buffer(Size: Positive) is new Runtime.Parent with record Data: Pointer; end record;','ada',3)
        self.assert_units('type Buffer\n(Size: Positive) is\nnew Runtime.Parent with\nrecord\nData: Pointer;\nend record;','ada',3)

    def test_ada_attribute_clause_is_declaration_not_for_loop(self):
        self.assert_units("for Bytes'Address use Buffer'Address;",'ada',1)
        self.assert_units("for Bytes'Address\nuse Buffer'Address;",'ada',1)
        self.assert_units("pragma Import(Ada,Bytes); for Bytes'Address use Buffer'Address;",'ada',2)
        self.assert_gate('for Item use record Field at 0 range 0..7; end record;','ada','foreign-ada-representation-clause-review')

    def test_multiline_independent_typed_names_match_expanded_declarations(self):
        for language in ('ada','pascal'):
            self.assert_units('A,\nB,\nC: Integer; ',language,3)
            self.assert_units('A: Integer;\nB: Integer;\nC: Integer;',language,3)
        self.assert_units('A,\nB: Integer := 0;','ada',2)
        self.assert_units('type Body=record x,y,z,\nvx,vy,vz,\nmass: Double; end;','pascal',9)
        self.assert_units('type Body=record\n'+''.join(name+': Double;\n' for name in ('x','y','z','vx','vy','vz','mass'))+'end;','pascal',9)

    def test_continued_groups_keep_formals_named_arguments_and_data_owned(self):
        self.assert_units('procedure P(A,\nB: Integer) is begin null; end P;','ada',3)
        self.assert_units('Call(\n Item=>A,\n Width=>B);','ada',3)
        self.assert_units('WriteLn(\n A,\n B);','pascal',3)
        self.assert_gate('A,\nB: array[0..2] of Integer;','pascal','foreign-multiline-declarator-review')
        metric=self.assert_gate('A,\nB: Vector(1..2) := (1,\n2);','ada','foreign-multiline-declarator-review')
        self.assertFalse(any(e['scope']=='foreign-continued-independent-declaration' for e in metric['canonical_events']))

    def test_ada_control_branch_and_written_closes(self):
        self.assert_units('procedure P is begin if A=B then X:=1; else X:=2; end if; end P;','ada',7)
        self.assert_units('procedure P is\nbegin\nif A=B then\nX:=1;\nelse\nX:=2;\nend if;\nend P;','ada',7)
        self.assert_units('procedure P is begin for I in 1..N loop X:=I; end loop; end P;','ada',5)

    def test_ada_case_choice_header_not_assignment(self):
        self.assert_units('procedure P is begin case X is when A|B => Y:=1; when others => Y:=2; end case; end P;','ada',8)

    def test_ada_exit_when_is_one_action_not_case_arm(self):
        self.assert_units('procedure P is begin loop exit when X>0; end loop; end P;','ada',5)
        self.assert_units('procedure P is\nbegin\nloop\nexit when X>0;\nend loop;\nend P;','ada',5)
        self.assert_units('exit Named_Loop when X>0;','ada',1)
        self.assert_units('exit;','ada',1)

    def test_ada_task_specification_counts_entries_not_formals(self):
        self.assert_units('task type Worker(A,B: Integer) is pragma Storage_Size(16); entry Start(X,Y: Integer); entry Done; end Worker;','ada',5)
        self.assert_units('task type Worker\n(A,B: Integer) is\npragma Storage_Size(16);\nentry Start\n(X,Y: Integer);\nentry Done;\nend Worker;','ada',5)
        self.assert_units('task type Worker(Index: Positive);','ada',1)
        self.assert_units('task Worker is end Worker;','ada',2)
        self.assert_gate('task type Worker is new Interface with end Worker;','ada','foreign-ada-task-interface-review')

    def test_ada_task_body_accept_scopes_have_written_closes(self):
        self.assert_units('task body Worker is x,y: Integer; begin accept Start(A,B: Integer) do x:=A; y:=B; end Start; accept Done; end Worker;','ada',9)
        self.assert_units('task body Worker is\nx: Integer;\ny: Integer;\nbegin\naccept Start\n(A,B: Integer) do\nx:=A;\ny:=B;\nend Start;\naccept Done;\nend Worker;','ada',9)
        self.assert_gate('task body Worker is begin null; end Other;','ada','foreign-ada-close-name-review')
        self.assert_gate('accept Start;','ada','foreign-ada-complex-owner-review')
        self.assert_gate('task body Worker is begin accept Start(x:=1; y:=2); end Worker;','ada','foreign-ada-complex-owner-review')
        self.assert_gate('task type Worker(x:=1; y:=2) is end Worker;','ada','foreign-ada-task-header-review')

    def test_ada_selective_accept_has_separate_branch_and_body_units(self):
        self.assert_units('task body Worker is begin loop select accept Start; or accept Done(X: out T) do X:=V; end Done; or terminate; end select; end loop; end Worker;','ada',13)
        self.assert_units('task body Worker is\nbegin\nloop\nselect\naccept Start;\nor\naccept Done(X: out T) do\nX:=V;\nend Done;\nor\nterminate;\nend select;\nend loop;\nend Worker;','ada',13)
        self.assert_units('task body Worker is begin select when Ready=>accept Start; or terminate; end select; end Worker;','ada',8)
        self.assert_gate('task body Worker is begin select accept Start; or delay 1.0; end select; end Worker;','ada','foreign-ada-select-variant-review')
        self.assert_gate('task body Worker is begin select Other.Start; else null; end select; end Worker;','ada','foreign-ada-select-variant-review')
        self.assert_units('task body Worker is begin select accept Start; Ready:=A or B; or accept Done; end select; end Worker;','ada',8)
        self.assert_units('task body Worker is begin select accept Start; Log(A or B); or accept Done; end select; end Worker;','ada',8)

    def test_pascal_header_begin_close_and_grouped_fields(self):
        self.assert_units('function F(X,Y: Integer): Integer; var A,B: Integer; begin A:=X; B:=Y; end;','pascal',6)
        self.assert_units('function F(X,Y: Integer): Integer;\nvar\nA: Integer;\nB: Integer;\nbegin\nA:=X;\nB:=Y;\nend;','pascal',6)
        self.assert_units('type Node=record Left,Right: Pointer; end;','pascal',4)
        self.assert_units('type\nNode=record\nLeft: Pointer;\nRight: Pointer;\nend;','pascal',4)

    def test_pascal_external_declaration_components_are_structural(self):
        self.assert_units("function F(A: Integer; B: Integer): Integer; cdecl; external 'lib' name 'f';",'pascal',1)
        self.assert_units("function F\n(A: Integer;\n B: Integer): Integer;\ncdecl;\nexternal 'lib' name 'f';",'pascal',1)

    def test_pascal_record_method_prototypes_keep_modifiers_in_header(self):
        self.assert_units('type Node=record Left,Right: Pointer; class function Check(const N: Pointer): Integer; static; inline; end;','pascal',5)
        self.assert_units('type Node=record\nLeft: Pointer;\nRight: Pointer;\nclass function Check(const N: Pointer): Integer;\nstatic;\ninline;\nend;','pascal',5)
        self.assert_units('class function Node.Check(const N: Pointer): Integer; begin Result:=1; end;','pascal',3)
        self.assert_units('class\nfunction Node.Check(const N: Pointer): Integer;\nbegin\nResult:=1;\nend;','pascal',3)
        self.assert_gate('type Node=class Left: Pointer; end;','pascal','foreign-pascal-complex-owner-review')

    def test_pascal_with_is_owned_control_with_written_body_close(self):
        self.assert_units('begin with Node^ do begin Left:=nil; Right:=nil; end; end.','pascal',5)
        self.assert_units('begin\nwith Node^ do\nbegin\nLeft:=nil;\nRight:=nil;\nend;\nend.','pascal',5)
        self.assert_units('begin with Node^ do if Right<>nil then Call; end.','pascal',4)

    def test_pascal_unbraced_else_and_bounded_loops(self):
        self.assert_units('begin if A=B then X:=1 else X:=2; end.','pascal',5)
        self.assert_units('begin if A=B then\nX:=1\nelse X:=2;\nend.','pascal',5)
        self.assert_units('begin for I:=1 to N do begin X:=I; end; end.','pascal',4)

    def test_pascal_repeat_until_written_close(self):
        self.assert_units('begin repeat X:=X+1; until X=3; end.','pascal',4)
        self.assert_units('begin\nrepeat\nX:=X+1;\nuntil X=3;\nend.','pascal',4)

    def test_pascal_case_and_bounds_vs_declarations(self):
        self.assert_units('begin case X of 1,2: Y:=1; else Y:=2; end; end.','pascal',7)
        self.assert_units('var A,B: array[0..3] of Integer; begin end.','pascal',3)
        self.assert_units('type Alias=Other;','pascal',1)
        self.assert_units('var A,B: Integer;','pascal',2)
        self.assert_units('var\nA: Integer;\nB: Integer;','pascal',2)

    def test_pascal_directives_are_meaningful_not_braces(self):
        self.assert_units('{$mode objfpc}{$H+}\nbegin X:=1; end.','pascal',4)
        self.assert_units("X:='{$mode objfpc}';",'pascal',1)

    def test_fortran_grouped_fields_and_entity_attributes(self):
        self.assert_units('type Body\nreal*8 x,y,z,vx,vy,vz,mass\nend type Body','fortran',9)
        self.assert_units('type Body\n'+'\n'.join('real*8 '+name for name in ('x','y','z','vx','vy','vz','mass'))+'\nend type Body','fortran',9)
        self.assert_units('integer, parameter :: n=3,m=4','fortran',2)
        self.assert_units('type(Node), pointer :: left,right','fortran',2)
        self.assert_units('character(len=8), dimension(2) :: words','fortran',1)

    def test_fortran_continued_entity_names_match_independent_declarations(self):
        self.assert_units('type(Node), pointer :: first=>null(), second=>null(), &\n third=>null()','fortran',3)
        self.assert_units('type(Node), pointer :: first=>null()\ntype(Node), pointer :: second=>null()\ntype(Node), pointer :: third=>null()','fortran',3)
        self.assert_units('type(c_ptr), intent(in), value :: context, callback, &\n data','fortran',3)
        self.assert_units('type(var_str) :: input, temp, sequences, &\n before, after','fortran',5)
        self.assert_units('integer :: first(2), &\n second(2)','fortran',2)
        self.assert_units('real, parameter :: values(2) = (/ &\n1.0,2.0 /)','fortran',2)

    def test_fortran_bind_c_derived_type_is_one_owned_header(self):
        self.assert_units('module M; type,bind(C)::mpz_t; integer d,s; end type mpz_t; end module M','fortran',6)
        self.assert_units('module M\ntype, bind(C) :: mpz_t\ninteger d\ninteger s\nend type mpz_t\nend module M','fortran',6)
        self.assert_gate('type, extends(Parent) :: Child\nend type Child','fortran','foreign-fortran-derived-type-attributes-review')

    def test_fortran_private_is_written_access_declaration_not_contains_section(self):
        self.assert_units('type,bind(C)::mpz_t; private; integer d,s; end type mpz_t','fortran',5)
        self.assert_units('type,bind(C)::mpz_t\nprivate\ninteger d\ninteger s\nend type mpz_t','fortran',5)
        self.assert_units('module M; private; contains; subroutine F(); end subroutine F; end module M','fortran',5)
        self.assert_gate('type T\npublic\nend type T','fortran','foreign-fortran-access-section-owner-review')

    def test_fortran_routine_header_formal_metadata_and_continuation(self):
        self.assert_units('real*8 function F( &\n X,Y) result(R)\nreal*8 X,Y\nR=X+Y\nend function F','fortran',5)
        self.assert_units('real*8 function F(X,Y) result(R)\nreal*8 X\nreal*8 Y\nR=X+Y\nend function F','fortran',5)

    def test_fortran_assignment_comparison_pointer_namedargs(self):
        self.assert_units('ok=(a==b); different=(a/=b); pointer=>target; call F(kind=8,n=2)','fortran',4)
        self.assert_units('integer, parameter :: n=kind(1.0d0)','fortran',1)

    def test_fortran_if_else_if_and_written_ends(self):
        self.assert_units('program P; if(a==b) then; x=1; else; x=2; endif; end program P','fortran',7)
        self.assert_units('program P\nif(a==b) then\nx=1\nelse\nx=2\nend if\nend program P','fortran',7)
        self.assert_units('program P\nif(a==b) x=1\nend program P','fortran',4)

    def test_fortran_do_header_and_openmp_directives(self):
        self.assert_units('program P; do i=1,n; x=x+1; enddo; end program P','fortran',5)
        self.assert_units('!$omp parallel do private(i)\ndo i=1,n\nx=x+1\nend do\n!$omp end parallel do','fortran',5)
        self.assert_units('!$OmP parallel do\ndo i=1,n\nx=x+1\nend do\n!$oMp end parallel do','fortran',5)

    def test_fortran_module_interface_type_written_closes(self):
        self.assert_units('module M\ninterface\ninteger function F(x) bind(C)\ninteger x\nend function\nend interface\nend module M','fortran',7)

    def test_fortran_contains_is_structural_without_fabricated_close(self):
        self.assert_units('program P\ncontains\nsubroutine F()\nend subroutine F\nend program P','fortran',4)
        self.assert_units('program P; contains; subroutine F(); end subroutine F; end program P','fortran',4)

    def test_fortran_named_construct_and_parameterless_subroutine(self):
        self.assert_units('program P\nreadFile: do\nx=x+1\nend do readFile\nend program P','fortran',5)
        self.assert_units('program P; outer: do; inner: do; x=x+1; end do inner; end do outer; end program P','fortran',7)
        self.assert_units('program P\ncontains\nsubroutine print_data\nx=1\nend subroutine print_data\nend program P','fortran',5)
        self.assert_units('subroutine F\ncheck: if(x>0) then\nx=1\nend if check\nend subroutine F','fortran',5)
        self.assert_gate('outer: do\nx=1\nend do other','fortran','foreign-fortran-construct-name-review')

    def test_literal_wrapping_and_comment_masks_keep_offsets(self):
        cases=[('ada','-- begin end\nX:=\"end;\";'),('pascal',"{ begin end }\nX:='end;';"),('fortran',"! end program\ns='end;'")]
        for language,source in cases:
            masked=mask_keyword(source,language)
            self.assertEqual(len(masked['cleaned']),len(source))
            self.assertEqual([i for i,c in enumerate(masked['cleaned']) if c=='\n'],[i for i,c in enumerate(source) if c=='\n'])
            self.assert_units(source,language,1)
        self.assert_units('real, parameter :: values(2) = (/ &\n1.0,2.0 /)','fortran',2)

    def test_complex_owners_require_review(self):
        self.assert_gate('task type T with Async_Readers;','ada','foreign-ada-complex-owner-review')
        self.assert_gate('type F is access function(X: Integer) return Integer;','ada','foreign-ada-complex-owner-review')
        self.assert_gate('unit U; interface implementation end.','pascal','foreign-pascal-complex-owner-review')
        self.assert_gate('do 100 i=1,n\n100 continue','fortran','foreign-fortran-labelled-do-review')
        self.assert_gate('#ifdef FEATURE\nx=1\n#endif','fortran','foreign-conditional-directive-review')

    def test_unknown_language_not_aliased(self):
        for language in ('c','julia','ruby','fsharp'):
            with self.assertRaises(ValueError): normalize('x=1;',language)
            with self.assertRaises(ValueError): classify_scopes('x=1;',language)


if __name__=='__main__':
    unittest.main()
