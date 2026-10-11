! The Computer Language Benchmarks Game
! https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
!
! converted to fortran by Gilbert Brietzke on 31. Januar 2011
! from C-code by Joseph Piché as a conversion
! from Java version by Oleg Mazurov and Isaac Gouy

program fannkuch

  implicit none

  integer :: n,checksum,maxFlipsCount
  character(len=2) :: arg
  character(len=10) :: out
  call get_command_argument(1,arg);read(arg,'(I2)')n
  call fannkuchredux(n,checksum,maxFlipsCount)
  write(out,'(I10)') checksum;
  write(*,'(A)') trim(adjustl(out))
  write(out,'(I10)') maxFlipsCount;
  write(*,'(A)') 'Pfannkuchen('//trim(adjustl(arg))//') = '//trim(adjustl(out))

contains

  subroutine fannkuchredux(n,checksum,maxFlipsCount)
    implicit none
    integer, intent(in) :: n
    integer, intent(out) :: maxFlipsCount,checksum
    integer :: perm(0:n-1),perm1(0:n-1),icount(0:n-1);
    integer :: i,r,temp,k,perm0,permCount = 0,flipsCount = 0;
    checksum = 0;  maxFlipsCount = 0;
    do i = 0,n-1
       perm1(i)= i;
    enddo
    r=n
    do while (.true.)
       do while (r /= 1)
          icount(r-1)=r; r=r-1;
       end do
       perm = perm1; flipsCount = 0;
       do while (perm(0)/=0) 
          k = perm(0)
          do  i=0,ishft(k+1,-1)-1
             temp = perm(i); perm(i) = perm(k-i); perm(k-i) = temp;
          enddo
          flipsCount = flipsCount + 1;
       end do
       maxFlipsCount = max(maxFlipsCount,flipsCount);
       if (modulo(permCount,2) == 0)then
          checksum = checksum + flipsCount
       else
          checksum = checksum - flipsCount
       end if
       do while (.true.)
          if (r==n) return
          perm0 = perm1(0); i = 0;
          do while (i < r) 
             perm1(i) = perm1(i + 1); i = i + 1;
          end do
          perm1(r) = perm0;
          icount(r) = icount(r) - 1;
          if (icount(r) > 0) exit;
          r = r + 1;
       end do
       permCount = permCount + 1;
    end do
  end subroutine fannkuchredux
  
end program fannkuch
