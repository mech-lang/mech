! Locally supplied Fortran implementation: full overlapping windows and growable maps.
module counter_map
  use iso_fortran_env, only: int8,int64
  implicit none
  character(len=*), parameter :: alphabet = 'ACGT'
  type hash_map
    integer(int64), allocatable :: keys(:), counts(:)
    integer :: used = 0
  end type
contains
  recursive integer function slot(m, key) result(pos)
    type(hash_map), intent(in) :: m
    integer(int64), intent(in) :: key
    pos = int(iand(ieor(key, shiftr(key,17)), int(size(m%keys)-1,int64))) + 1
    do while (m%keys(pos) /= -1_int64 .and. m%keys(pos) /= key)
      pos = mod(pos,size(m%keys)) + 1
    end do
  end function
  recursive subroutine grow(m)
    type(hash_map), intent(inout) :: m
    type(hash_map) :: next
    integer :: i, p, capacity
    capacity = 16
    if (allocated(m%keys)) capacity = 2 * size(m%keys)
    allocate(next%keys(capacity), next%counts(capacity))
    next%keys = -1_int64
    next%counts = 0_int64
    if (allocated(m%keys)) then
      do i=1,size(m%keys)
        if (m%keys(i) == -1_int64) cycle
        p = slot(next,m%keys(i))
        next%keys(p) = m%keys(i)
        next%counts(p) = m%counts(i)
      end do
    end if
    call move_alloc(next%keys,m%keys)
    call move_alloc(next%counts,m%counts)
  end subroutine
  recursive subroutine increment(m,key)
    type(hash_map), intent(inout) :: m
    integer(int64), intent(in) :: key
    integer :: p
    if (.not. allocated(m%keys)) call grow(m)
    if (10 * (m%used+1) > 7 * size(m%keys)) call grow(m)
    p = slot(m,key)
    if (m%keys(p) == -1_int64) then
      m%keys(p) = key
      m%used = m%used+1
    end if
    m%counts(p) = m%counts(p)+1_int64
  end subroutine
  recursive integer(int64) function lookup(m,key) result(value)
    type(hash_map), intent(in) :: m
    integer(int64), intent(in) :: key
    integer :: p
    value = 0_int64
    if (.not. allocated(m%keys)) return
    p = slot(m,key)
    if (m%keys(p) == key) value = m%counts(p)
  end function
  recursive subroutine count_windows(sequence,n,k,target,frequencies,answer)
    integer(int8), intent(in) :: sequence(:)
    character(len=*), intent(in) :: target
    integer, intent(in) :: n,k
    integer(int64), intent(out) :: frequencies(16),answer
    type(hash_map) :: m
    integer(int64) :: key,mask
    integer :: i,symbol
    key = 0_int64
    mask = ishft(1_int64,2*k)-1_int64
    do i=1,n
      symbol = int(sequence(i))
      if (symbol < 0) error stop 'Non-DNA input'
      key = iand(ior(ishft(key,2),int(symbol,int64)),mask)
      if (i >= k) call increment(m,key)
    end do
    frequencies = 0_int64
    answer = 0_int64
    if (k <= 2) then
      do i=0,4**k-1
        frequencies(i+1) = lookup(m,int(i,int64))
      end do
    else
      key = 0_int64
      do i=1,k
        key = ior(ishft(key,2),int(index(alphabet,target(i:i))-1,int64))
      end do
      answer = lookup(m,key)
    end if
  end subroutine
end module
program knucleotide
  use iso_fortran_env, only: int8,int64
  use counter_map
  implicit none
  character(len=:), allocatable :: dna
  integer(int8), allocatable :: encoded(:)
  integer, parameter :: widths(7) = [1,2,3,4,6,12,18]
  character(len=18), parameter :: targets(7) = [character(len=18) :: &
    '', '', 'GGT', 'GGTA', 'GGTATT', 'GGTATTTTAATT', 'GGTATTTTAATTTATAGT']
  integer(int64) :: counts(16,7), answers(7)
  integer :: input_unit, input_size, i, j, start, finish, used, job, code
  integer :: order(16), limit, temporary, a, b
  character(len=2) :: word
  character(len=32) :: percentage
  open(newunit=input_unit,file='/dev/stdin',access='stream',form='unformatted',status='old')
  inquire(unit=input_unit,size=input_size)
  allocate(character(len=input_size) :: dna)
  read(input_unit) dna
  close(input_unit)
  start = 1
  do while (start <= input_size)
    finish = index(dna(start:),achar(10))
    if (finish == 0) finish = input_size-start+2
    finish = start+finish-1
    if (index(dna(start:finish-1),'>THREE') == 1) exit
    start = finish+1
  end do
  used = 0
  do i=finish+1,input_size
    if (dna(i:i) == '>') exit
    code = iachar(dna(i:i))
    if (code == 10 .or. code == 13) cycle
    if (code >= iachar('a') .and. code <= iachar('z')) code = code-32
    used = used+1
    code = index(alphabet,achar(code))-1
    if (code < 0) error stop "Non-DNA input"
    dna(used:used) = achar(code)
  end do
  allocate(encoded(used))
  do i=1,used
    encoded(i) = int(iachar(dna(i:i)),int8)
  end do
  deallocate(dna)
  !$omp parallel do num_threads(7)
  do job=1,7
    call count_windows(encoded,used,widths(job),targets(job),counts(:,job),answers(job))
  end do
  !$omp end parallel do
  do job=1,2
    limit = 4**widths(job)
    order(:limit) = [(i,i=1,limit)]
    do i=2,limit
      temporary = order(i)
      j = i-1
      do while (j >= 1)
        a = order(j)
        b = temporary
        if (counts(a,job) > counts(b,job)) exit
        if (counts(a,job) == counts(b,job) .and. a < b) exit
        order(j+1) = order(j)
        j = j-1
      end do
      order(j+1) = temporary
    end do
    do i=1,limit
      code = order(i)-1
      if (counts(code+1,job) == 0) cycle
      do j=widths(job),1,-1
        word(j:j) = alphabet(mod(code,4)+1:mod(code,4)+1)
        code = code/4
      end do
      write(percentage,'(F20.3)') 100.0d0*counts(order(i),job)/(used-widths(job)+1)
      write(*,'(A,1X,A)') word(:widths(job)), trim(adjustl(percentage))
    end do
    print '(A)', ''
  end do
  do job=3,7
    write(*,'(I0,A,A)') answers(job),achar(9),trim(targets(job))
  end do
end program
