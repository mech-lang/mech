! The Computer Language Benchmarks Game
! https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
!
! Author: Jannis Teunissen
! Heavily based on Fortran code contributed by Pascal Parois
!
! Compilation:
! gfortran -pipe -O3 -fomit-frame-pointer -march=core2 \
!          -fopenmp mandelbrot.f90 -o mandelbrot
! ifort -O3 -march=core2 -qopenmp mandelbrot.f90 -o mandelbrot
program mandelbrot
  use iso_fortran_env, dp => real64

  implicit none

  integer, parameter  :: max_iterations = 50 ! How many Mandelbrot iterations
  integer, parameter  :: buf_size       = 32 ! Buffer size for arrays
  real(dp), parameter :: threshold      = 4.0_dp ! Threshold for iterations

  integer                       :: i, j, k, n, pos, my_unit
  integer                       :: bytepos, nx, width
  integer(int8), allocatable    :: buffer(:, :)
  real(dp), dimension(buf_size) :: cr, ci, zr, zi, tr, ti
  real(dp)                      :: cte
  real(dp), allocatable         :: cr_values(:), ci_values(:)
  character(len=64)             :: argv, header
  character(len=:), allocatable :: char_output
  character(len=*), parameter   :: output_style = "multiple_strings"

  ! Get bitmap size from command line argument
  call get_command_argument(1, argv)
  read(argv, *) width

  ! Determine smallest multiple of buf_size greater or equal to width
  nx = ceiling(width/real(buf_size, dp)) * buf_size

  ! Allocate results buffer
  allocate(buffer(nx/8, width))
  buffer = 0

  ! Set complex coordinates from [-1.5 - i -- 0.5 + i]
  allocate(cr_values(nx))
  allocate(ci_values(width))
  cte = 2.0_dp/width ! Step size
  cr_values = [(cte * i - 1.5_dp, i = 0, nx-1)]
  ci_values = [(cte * i - 1, i = 0, width-1)]

  !$omp parallel do private(i, j, k, n, bytepos, pos) &
  !$omp private(zr, zi, cr, ci, tr, ti) schedule(guided)
  do i = 1, width
     pos     = 1
     bytepos = 8
     ci      = ci_values(i)

     do j = 1, nx, buf_size
        ! Set initial values (together with ci above)
        cr = cr_values(j:j+buf_size-1)
        zr = 0.0_dp
        zi = 0.0_dp
        tr = 0.0_dp
        ti = 0.0_dp

        ! Perform very simple loop unrolling
        do k = 1, max_iterations/2
           ! Update the real and complex parts
           zi = 2*zr*zi+ci
           zr = tr-ti+cr
           ti = zi*zi
           tr = zr*zr
           ! One more update
           zi = 2*zr*zi+ci
           zr = tr-ti+cr
           ti = zi*zi
           tr = zr*zr

           ! Check if all values have exceeded the threshold
           if (all(tr + ti > threshold)) exit
        end do

        ! Store results in buffer
        do k = 1, buf_size
           ! Pack boolean values into bytes
           bytepos = bytepos - 1
           if (tr(k)+ti(k) < threshold .and. j <= width) then
              buffer(pos, i) = ibset(buffer(pos, i), bytepos)
           end if

           ! Advance at the end of the byte
           if (bytepos == 0) then
              bytepos = 8
              pos     = pos + 1
           end if
        end do
     end do
  end do
  !$omp end parallel do

  ! pbm header
  write(header, "(a,i0,a,i0,a)") "P4" // new_line("a"), &
       width, " ", width, new_line("a")

  i = ceiling(width/8.0_dp)

  ! Fortran allows to write binary output (stream access) to files, but not to
  ! stdout. Some tricks are therefore required.

  select case (output_style)
  case ("one_big_string")
     ! Convert binary data to one big string
     allocate(character(len=i*width) :: char_output)
     char_output = transfer(buffer(1:i, :), char_output)
     write(*, '(a)', advance='no') trim(header)
     write(*, '(a)', advance='no') char_output
  case ("multiple_strings")
     ! This avoids problems with ifort for larger output sizes
     allocate(character(len=i) :: char_output)
     write(*, '(a)', advance='no') trim(header)
     do j = 1, width
        char_output = transfer(buffer(1:i, j), char_output)
        write(*, '(a)', advance='no') char_output
     end do
  case default
     ! Write unformatted data to stdout, by opening it as a file
     open(newunit=my_unit, file="/dev/stdout", access='stream', &
          form='unformatted', action='write')
     write(my_unit) trim(header)
     write(my_unit) buffer(1:i, :)
     flush(my_unit)
     close(my_unit)
  end select

end program mandelbrot
