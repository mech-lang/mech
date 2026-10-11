! The Computer Language Benchmarks Game
! https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
!
! contributed by Pascal Parois

program mandelbrot 
implicit none
    
integer, parameter :: iter = 50, vsize=32
integer, parameter :: int8 = selected_int_kind(2)
integer, parameter :: dp = selected_real_kind(15, 307)

integer :: i, j,k, pos
real(dp),dimension(vsize) :: cr,ci,zr,zi,tr,ti
real(dp) cte
integer :: bytepos, grid
integer(int8) :: byte
integer(int8), dimension(:,:), allocatable :: buf
integer w
character(len=8) :: argv
character(len=100) :: sbuffer

real(dp), parameter :: threshold = 4.0_dp

    call get_command_argument(1, argv)
    read(argv, *) w        
    grid=ceiling(w/real(vsize))*vsize

    allocate(buf(grid/8, w))
    buf=0.0_dp

    cte=2.0_dp/real(w,dp)

    !$omp parallel do default(none) shared(grid, buf, cte, w)&
    !$omp& private(i, j, k, bytepos, pos, byte)&
    !$omp& private(zr, zi, cr, ci, tr, ti)&
    !$omp& schedule(guided)
    do i=0, w-1
        pos=0
        byte=0_int8
        bytepos=8
        do j = 0, grid-1,vsize
            ci = cte*real(i,dp)-1.0_dp
            do k=0, vsize-1
                cr(k+1) = cte*real(j+k,dp)-1.5_dp
            end do

            zr=0.0_dp
            zi=0.0_dp
            tr=0.0_dp
            ti=0.0_dp

            do k=1, iter
                zi=2.0_dp*zr*zi+ci
                zr=tr-ti+cr
                ti=zi*zi
                tr=zr*zr

                if (all(tr+ti>threshold)) then 
                    exit
                end if 
            end do

            do k=1, vsize
                bytepos=bytepos-1
                if (.not. isnan(tr(k)+ti(k)) .and. tr(k)+ti(k)<threshold .and. j<=w) then
                    byte = ibset(byte, bytepos)
                end if
                if(bytepos==0) then
                    bytepos=8
                    pos = pos + 1
                    buf(pos,i+1) = byte                         
                    byte=0_int8
                end if
            end do
        end do  
    end do
    !$omp end parallel do


    !open(unit=100,file='out',status='replace', form='unformatted',access='stream')
    !! pbm header
    !write(100) "P4"//char(10)
    !write(sbuffer, '(I0,A,I0,A)') w,' ',w,char(10)
    !write(100) trim(sbuffer)
    !
    !do i = 1, w
    !    write(100) buf(1:ceiling(w/8.0),i)
    !end do
    !close(100)

    ! pbm header
    write(*,'("P4",/,i0," ",i0)') w, w
    ! print output
    do i = 1, w
        write(*, '(10000000a1)', advance='no') buf(1:ceiling(w/8.0),i)
    end do
          
end program mandelbrot
