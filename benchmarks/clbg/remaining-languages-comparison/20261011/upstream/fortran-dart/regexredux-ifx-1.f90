! The Computer Language Benchmarks Game
! https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
!
! Contributed by Tyler Funnell
! translation of GCC #5 by Jeremy Zerfas & Zoltan Herczeg
! compilation: ifort -O3 -xHost -ipo -qopenmp regexredux.f90


module pcre2_mod
    use iso_c_binding
    implicit none

    type var_str
        character(len=:), allocatable :: data
        integer :: size
    end type

    integer(c_int), parameter :: PCRE2_JIT_COMPLETE = int(z'00000001')

    interface
        ! void pcre2_code_free(pcre2_code *code)
        subroutine pcre2_code_free(code) bind(c, name='pcre2_code_free_8')
            import
            type(c_ptr), intent(in), value :: code
        end subroutine

        ! pcre2_code *pcre2_compile(PCRE2_SPTR pattern, PCRE2_SIZE length,
        !                           uint32_t options, int *errorcode,
        !                           PCRE2_SIZE *erroroffset,
        !                           pcre2_compile_context *ccontext)
        function pcre2_compile &
                (pattern, length, options, errorcode, erroroffset, ccontext) &
                result(code) bind(c, name='pcre2_compile_8')
            import
            type(c_ptr) :: code
            character(kind=c_char), intent(in) :: pattern(*)
            integer(c_int), intent(in), value :: length, options
            integer(c_int), intent(out) :: errorcode
            integer(c_size_t), intent(out) :: erroroffset
            type(c_ptr), intent(in), value :: ccontext
        end function

        ! PCRE2_SIZE *pcre2_get_ovector_pointer(pcre2_match_data *match_data)
        function pcre2_get_ovector_pointer(match_data) result(ret) &
                bind(c, name='pcre2_get_ovector_pointer_8')
            import
            type(c_ptr) :: ret
            type(c_ptr), intent(in), value :: match_data
        end function

        ! int pcre2_jit_compile(pcre2_code *code, uint32_t options)
        function pcre2_jit_compile(code, options) &
                result(errorcode) bind(c, name='pcre2_jit_compile_8')
            import
            integer :: errorcode
            type(c_ptr), intent(in), value :: code
            integer(c_int), intent(in), value :: options
        end function

        ! int pcre2_jit_match(const pcre2_code *code, PCRE2_SPTR subject,
        !                     PCRE2_SIZE length, PCRE2_SIZE startoffset,
        !                     uint32_t options, pcre2_match_data *match_data,
        !                     pcre2_match_context *mcontext)
        function pcre2_jit_match (code, subject, length, startoffset, options, &
                match_data, mcontext) &
                result(ret) bind(c, name='pcre2_jit_match_8')
            import
            integer :: ret
            type(c_ptr), intent(in), value :: code, match_data, mcontext
            character(len=1, kind=c_char), intent(in) :: subject(*)
            integer(c_int), intent(in), value :: length, startoffset, options
        end function

        ! void pcre2_jit_stack_assign(pcre2_match_context *mcontext,
        !   pcre2_jit_callback callback_function, void *callback_data)
        subroutine pcre2_jit_stack_assign &
                (mcontext, callback_function, callback_data) &
                bind(c, name='pcre2_jit_stack_assign_8')
            import
            type(c_ptr), intent(in), value :: mcontext, callback_function, &
                callback_data
        end subroutine

        ! pcre2_jit_stack *pcre2_jit_stack_create(
        !   PCRE2_SIZE startsize, PCRE2_SIZE maxsize,
        !   pcre2_general_context *gcontext)
        function pcre2_jit_stack_create(startsize, maxsize, gcontext) &
                result(stack) bind(c, name='pcre2_jit_stack_create_8')
            import
            type(c_ptr) :: stack
            integer(c_size_t), intent(in), value :: startsize, maxsize
            type(c_ptr), intent(in), value :: gcontext
        end function

        ! void pcre2_jit_stack_free(pcre2_jit_stack *jit_stack)
        subroutine pcre2_jit_stack_free(jit_stack) &
                bind(c, name='pcre2_jit_stack_free_8')
            import
            type(c_ptr), intent(in), value :: jit_stack
        end subroutine

        ! pcre2_match_context *pcre2_match_context_create(
        !   pcre2_general_context *gcontext)
        function pcre2_match_context_create(gcontext) &
                result(mcontext) bind(c, name='pcre2_match_context_create_8')
            import
            type(c_ptr) :: mcontext
            type(c_ptr), intent(in), value :: gcontext
        end function

        ! void pcre2_match_context_free(pcre2_match_context *mcontext)
        subroutine pcre2_match_context_free(mcontext) &
                bind(c, name='pcre2_match_context_free_8')
            import
            type(c_ptr), intent(in), value :: mcontext
        end subroutine

        ! pcre2_match_data *pcre2_match_data_create(
        !   uint32_t ovecsize, pcre2_general_context *gcontext)
        function pcre2_match_data_create(ovecsize, gcontext) &
                result(mdata) bind(c, name='pcre2_match_data_create_8')
            import
            type(c_ptr) :: mdata
            integer(c_size_t), intent(in), value :: ovecsize
            type(c_ptr), intent(in), value :: gcontext
        end function

        ! void pcre2_match_data_free(pcre2_match_data *match_data)
        subroutine pcre2_match_data_free(match_data) &
                bind(c, name='pcre2_match_data_free_8')
            import
            type(c_ptr), intent(in), value :: match_data
        end subroutine
    end interface

    contains
        subroutine replace(pattern, replacement, src_string, dst_string, &
                mcontext, mdata)
            character(len=*), intent(in) :: pattern, replacement
            type(var_str), intent(in) :: src_string
            type(var_str), intent(inout) :: dst_string
            type(c_ptr), intent(in) :: mcontext, mdata

            character(len=:), allocatable :: temp

            type(c_ptr) :: regex, c_match
            integer(c_size_t), pointer :: match(:)
            integer(c_int) :: errorcode, pos
            integer(c_size_t) :: erroroffset


            c_match = pcre2_get_ovector_pointer(mdata)
            call c_f_pointer(c_match, match, [2])

            regex = pcre2_compile(pattern, len(pattern), 0_c_int, errorcode, &
                erroroffset, c_null_ptr)
            errorcode = pcre2_jit_compile(regex, PCRE2_JIT_COMPLETE)

            pos = 0

            ! Find each match of the pattern in src_string and append the
            ! characters preceding each match and the replacement text to
            ! dst_string.
            do
                errorcode = pcre2_jit_match(regex, src_string%data, &
                    src_string%size, pos, 0, mdata, mcontext)

                if (errorcode < 0) exit

                ! Allocate more memory for dst_string if there is not enough
                ! space for the characters preceding the match and the
                ! replacement text.
                do
                    if (dst_string%size + match(1) - pos + len(replacement) <= &
                        len(dst_string%data)) exit

                    allocate (character(len(dst_string%data) * 2) :: temp)
                    temp(1:dst_string%size) = dst_string%data(:dst_string%size)
                    call move_alloc(temp, dst_string%data)
                end do

                ! Append the characters preceding the match and the replacement
                ! text to dst_string and update the size of dst_string.
                dst_string%data( &
                    (dst_string%size + 1):(dst_string%size + match(1) - pos) &
                ) = src_string%data((pos + 1):match(1))
                dst_string%size = dst_string%size + match(1) - pos

                dst_string%data( &
                    (dst_string%size + 1):(dst_string%size + len(replacement)) &
                ) = replacement
                dst_string%size = dst_string%size + len(replacement)

                ! Update pos to continue searching after the current match.
                pos = match(2)
            end do

        call pcre2_code_free(regex)

        ! Allocate more memory for dst_string if there is not enough space for
        ! the characters following the last match (or the entire src_string if
        ! there was no match).
        do
            if ( &
                dst_string%size + src_string%size - pos <= &
                len(dst_string%data) & 
            ) exit

            allocate (character(len(dst_string%data) * 2) :: temp)
            temp(1:dst_string%size) = dst_string%data(:dst_string%size)
            call move_alloc(temp, dst_string%data)
        end do

        ! Append the characters following the last match (or the entire
        ! src_string if there was no match) to dst_string and update the size
        ! of dst_string.
        dst_string%data( &
            (dst_string%size + 1):(dst_string%size + src_string%size - pos) &
        ) = src_string%data((pos + 1):src_string%size)
        dst_string%size = dst_string%size + src_string%size - pos

        end subroutine
end module pcre2_mod

program regexredux
    use iso_fortran_env
    use iso_c_binding
    use pcre2_mod
    implicit none

    type const_str
        character(len=:), allocatable :: data
    end type

    type(const_str), dimension(:), allocatable :: count_info
    type(const_str), dimension(:, :), allocatable :: replace_info

    type(var_str) :: input, temp, sequences, prereplace_string, &
        postreplace_string
    integer :: input_size, info_count, i

    type(c_ptr) :: mdata, mcontext, stack, regex, c_match
    integer(c_size_t), pointer :: match(:)
    integer(c_size_t) :: erroroffset
    integer(c_int) :: errorcode, pos


    count_info = [ &
        const_str('agggtaaa|tttaccct'), &
        const_str('[cgt]gggtaaa|tttaccc[acg]'), &
        const_str('a[act]ggtaaa|tttacc[agt]t'), &
        const_str('ag[act]gtaaa|tttac[agt]ct'), &
        const_str('agg[act]taaa|ttta[agt]cct'), &
        const_str('aggg[acg]aaa|ttt[cgt]ccct'), &
        const_str('agggt[cgt]aa|tt[acg]accct'), &
        const_str('agggta[cgt]a|t[acg]taccct'), &
        const_str('agggtaa[cgt]|[acg]ttaccct') &
    ]
    replace_info = reshape([ &
        const_str('tHa[Nt]'), const_str('<4>'), &
        const_str('aND|caN|Ha[DS]|WaS'), const_str('<3>'), &
        const_str('a[NSt]|BY'), const_str('<2>'), &
        const_str('<[^>]*>'), const_str('|'), &
        const_str('\|[^|][^|]*\|'), const_str('-') &
    ], [2, 5])

    open(unit=input_unit, action="read", form="unformatted", access="stream")
    inquire(unit=input_unit, size=input_size)
    allocate(character(len=input_size) :: input%data)
    read(input_unit) input%data
    close(input_unit)

    input%size = input_size

    !$omp parallel private(mcontext, stack, mdata)
    mcontext = pcre2_match_context_create(c_null_ptr)
    stack = pcre2_jit_stack_create(16384_c_size_t, 16384_c_size_t, c_null_ptr)
    call pcre2_jit_stack_assign(mcontext, c_null_ptr, stack)
    mdata = pcre2_match_data_create(16_c_size_t, c_null_ptr)

    !$omp single
    allocate (character(len=16384) :: sequences%data)
    call replace('>.*\n|\n', '', input, sequences, mcontext, mdata)
    deallocate(input%data)
    !$omp end single

    ! We'll use two strings when doing all the replacements, searching
    ! for patterns in prereplace_string and using postreplace_string to
    ! store the string after the replacements have been made. After
    ! each iteration these two then get swapped. Start out with both
    ! strings having the same capacity as the sequences string and also
    ! copy the sequences string into prereplace_string for the initial
    ! iteration.
    !$omp single
    allocate(character(len=len(sequences%data)) :: prereplace_string%data)
    allocate(character(len=len(sequences%data)) :: postreplace_string%data)
    prereplace_string%data(:) = sequences%data(:)
    prereplace_string%size = sequences%size
    postreplace_string%size = 0

    ! Iterate through all the replacement patterns and their
    ! replacements in replace_info.
    do i = 1, size(replace_info, 2)
        call replace( &
            replace_info(1, i)%data, replace_info(2, i)%data, &
            prereplace_string, postreplace_string, mcontext, mdata)

        call move_alloc(prereplace_string%data, temp%data)
        call move_alloc(postreplace_string%data, prereplace_string%data)
        prereplace_string%size = postreplace_string%size
        call move_alloc(temp%data, postreplace_string%data)
        postreplace_string%size = 0
    end do

    deallocate(prereplace_string%data)
    deallocate(postreplace_string%data)
    !$omp end single nowait

    ! Iterate through all the count patterns in count_info[] and perform
    ! the counting for each one on a different thread if available.
    !$omp do schedule(dynamic) ordered &
    !$omp private(errorcode, erroroffset, info_count, pos, regex, c_match, match)
    do i = 1, size(count_info)
        errorcode = 0
        info_count = 0
        pos = 0

        c_match = pcre2_get_ovector_pointer(mdata)
        call c_f_pointer(c_match, match, [2])

        ! Compile and study pattern.
        regex = pcre2_compile( &
            count_info(i)%data, len(count_info(i)%data), &
            0_c_int, errorcode, erroroffset, c_null_ptr)
        errorcode = pcre2_jit_compile(regex, PCRE2_JIT_COMPLETE)

        ! Find each match of the pattern in the sequences string and
        ! increment count for each match.
        do
            errorcode = pcre2_jit_match( &
                regex, sequences%data, sequences%size, pos, 0, mdata, mcontext)

            if (errorcode < 0) exit

            info_count = info_count + 1
            pos = match(2)
        end do

        call pcre2_code_free(regex)

        ! Print the count for each pattern in the correct order.
        !$omp ordered
        print '(A, " ", I0)', count_info(i)%data, info_count
        !$omp end ordered
    end do
    !$omp end do

    call pcre2_match_context_free(mcontext)
    call pcre2_jit_stack_free(stack)
    call pcre2_match_data_free(mdata)
    !$omp end parallel

    deallocate(sequences%data)

    ! Print the size of the original input, the size of the input without the
    ! sequence descriptions & new lines, and the size after having made all the
    ! replacements.
    print '(/ I0 / I0 / I0)', input_size, sequences%size, prereplace_string%size

end program regexredux
