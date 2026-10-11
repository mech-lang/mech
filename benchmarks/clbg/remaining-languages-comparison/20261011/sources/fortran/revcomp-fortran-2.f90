! The Computer Language Benchmarks Game
! https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
!
! contributed by Steve Decker, modified from the version by Simon Geard
! compilation:
!   g95 -O3 reverse.f90
!   ifort -ipo -O3 -static reverse.f90

program revcomp
  implicit none

  character, parameter :: EndStr = ">"
  integer,   parameter :: LineWidth = 60

  character(len=LineWidth), dimension(:), allocatable :: data, w
  logical                  :: insection = .false.
  integer                  :: stat, bcount
  character(len=LineWidth) :: line, title

  ! Read and process
  allocate(data(100))  ! Allocate enough lines so that we don't have to grow the array for the test
  readFile: do
     read(*, "(a)", iostat=stat) line
     if (stat /= 0) exit readFile
     if (line(1:1) == EndStr) then
        if (insection) then
           write(*, "(a)") trim(title)
           call printReverseFasta
        else
           insection = .true.
        end if
        title = line
        bcount = 0
        cycle readFile
     end if
     bcount = bcount + 1
     if (bcount > size(data)) then ! Included for completeness - it shouldn't be called in the test
        allocate(w(size(data)))
        w = data
        deallocate(data)
        allocate(data(2*size(w)))
        data(1:size(w)) = w
        deallocate(w)
     end if
     data(bcount) = line
  end do readFile

  write(*, "(a)") trim(title)
  call printReverseFasta
  
contains

  subroutine printReverseFasta
    ! Output the data in reverse order and with the complement
    character, dimension(65:121), parameter :: Complement = (/ "T", "V", "G", &
         "H", "E", "F", "C", "D", "I", "J", "M", "L", "K", "N", "O", "P",  &
         "Q", "Y", "S", "A", "A", "B", "W", "X", "R", (" ", stat = 90, 96),  &
         "T", "V", "G", "H", "E", "F", "C", "D", "I", "J", "M", "L", "K",  &
         "N", "O", "P", "Q", "Y", "S", "A", "A", "B", "W", "X", "R" /)

    integer :: fLine, fChar, bLine, bChar
    character :: c

    fLine = 1
    fChar = 1
    bLine = bcount
    bChar = len_trim(data(bLine))
    do
       if (fLine > bLine .or. fLine == bLine .and. fChar >= bChar) exit
       c = data(fLine)(fChar:fChar)
       data(fLine)(fChar:fChar) = Complement(iachar(data(bLine)(bChar:bChar)))
       data(bLine)(bChar:bChar) = Complement(iachar(c))
       fChar = fChar + 1
       if (fChar > LineWidth) then
          fChar = 1
          fLine = fLine + 1
       end if
       bChar = bChar - 1
       if (bChar == 0) then
          bChar = LineWidth
          bLine = bLine - 1
       end if
    end do
    if (fLine == bLine .and. fChar == bChar)  &
         data(fLine)(fChar:fChar) = Complement(iachar(data(fLine)(fChar:fChar)))
    do fLine = 1, bcount-1
       write(*, "(a)") data(fLine)
    end do
    write(*, "(a)") trim(data(bcount))
  end subroutine printReverseFasta
end program revcomp
