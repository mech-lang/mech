! The Computer Language Benchmarks Game
! https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
!
!   contributed by Simon Geard, translated from  Mark C. Williams nbody.java
!
! ifort -O3 -static-libcxa -o nbody nbody.f90
!

program nbody

  implicit none
  integer result, num, i, k
  character(len=8) argv
  real*8, parameter :: tstep = 0.01d0
  real*8, parameter ::  PI = 3.141592653589793d0
  real*8, parameter ::  SOLAR_MASS = 4 * PI * PI
  real*8, parameter ::  DAYS_PER_YEAR = 365.24d0
  real*8 :: e
  type body
     real*8 x, y, z, vx, vy, vz, mass
  end type body
  type(body), parameter :: jupiter = body( &
       4.84143144246472090d0,    -1.16032004402742839d0, &
       -1.03622044471123109d-01, 1.66007664274403694d-03 * DAYS_PER_YEAR, &
       7.69901118419740425d-03 * DAYS_PER_YEAR, -6.90460016972063023d-05 * DAYS_PER_YEAR, &
       9.54791938424326609d-04 * SOLAR_MASS)

  type(body), parameter :: saturn = body( &
       8.34336671824457987d+00, &
       4.12479856412430479d+00, &
       -4.03523417114321381d-01, &
       -2.76742510726862411d-03 * DAYS_PER_YEAR, &
       4.99852801234917238d-03 * DAYS_PER_YEAR, &
       2.30417297573763929d-05 * DAYS_PER_YEAR, &
       2.85885980666130812d-04 * SOLAR_MASS)

  type(body), parameter :: uranus = body( &
	   1.28943695621391310d+01, &
	   -1.51111514016986312d+01, &
	   -2.23307578892655734d-01, &
	   2.96460137564761618d-03 * DAYS_PER_YEAR, &
	   2.37847173959480950d-03 * DAYS_PER_YEAR, &
	   -2.96589568540237556d-05 * DAYS_PER_YEAR, &
	   4.36624404335156298d-05 * SOLAR_MASS )

  type(body), parameter :: neptune = body( &
       1.53796971148509165d+01, &
       -2.59193146099879641d+01, &
       1.79258772950371181d-01, &
       2.68067772490389322d-03 * DAYS_PER_YEAR, &
       1.62824170038242295d-03 * DAYS_PER_YEAR, &
       -9.51592254519715870d-05 * DAYS_PER_YEAR, &
       5.15138902046611451d-05 * SOLAR_MASS)

  type(body), parameter :: sun = body(0.0d0, 0.0d0, 0.0d0, 0.0d0, 0.0d0, 0.0d0, SOLAR_MASS)

  type(body), dimension(5) :: bodies
  bodies = (/ sun, jupiter, saturn, uranus, neptune /)

  call getarg(1,argv)
  read(argv,*) num

  call offsetMomentum(1,bodies)
  e = energy(bodies)
  write(*,'(f12.9)') e
  do i=1,num
     call advance(tstep, bodies)
  end do
  e = energy(bodies)
  write(*,'(f12.9)') e

contains

  subroutine offsetMomentum(k, bodies)
    integer, intent(in) :: k
    type(body), dimension(:), intent(inout) :: bodies
    real*8 :: px, py, pz
    px = 0.0d0
    py = 0.0d0
    pz = 0.0d0
    do i=1,size(bodies)
       px = px + bodies(i)%vx * bodies(i)%mass;
       py = py + bodies(i)%vy * bodies(i)%mass;
       pz = pz + bodies(i)%vz * bodies(i)%mass;
    end do
    bodies(k)%vx = -px/SOLAR_MASS
    bodies(k)%vy = -py/SOLAR_MASS
    bodies(k)%vz = -pz/SOLAR_MASS
  end subroutine offsetMomentum


  subroutine advance(tstep, bodies)
  real*8, intent(in) :: tstep
  type(body), dimension(:), intent(inout) :: bodies

  real*8 dx, dy, dz, distance, mag
  integer i, j
  
  do i=1,size(bodies)
     do j=i+1,size(bodies)
        dx = bodies(i)%x - bodies(j)%x
        dy = bodies(i)%y - bodies(j)%y
        dz = bodies(i)%z - bodies(j)%z
        
        distance = sqrt(dx*dx + dy*dy + dz*dz)
        mag = tstep / (distance * distance * distance)
        
        bodies(i)%vx = bodies(i)%vx - dx * bodies(j)%mass * mag
        bodies(i)%vy =  bodies(i)%vy - dy * bodies(j)%mass * mag
        bodies(i)%vz =  bodies(i)%vz - dz * bodies(j)%mass * mag
        
        bodies(j)%vx = bodies(j)%vx + dx * bodies(i)%mass * mag
        bodies(j)%vy = bodies(j)%vy + dy * bodies(i)%mass * mag
        bodies(j)%vz = bodies(j)%vz + dz * bodies(i)%mass * mag
     end do
  end do
     
  do i=1,size(bodies)
     bodies(i)%x = bodies(i)%x + tstep * bodies(i)%vx
     bodies(i)%y = bodies(i)%y + tstep * bodies(i)%vy
     bodies(i)%z = bodies(i)%z + tstep * bodies(i)%vz
  end do

  end subroutine advance

  real*8 function energy(bodies)
    type(body), dimension(:), intent(in) :: bodies
    real*8 dx, dy, dz, distance
    integer i, j

    energy = 0.0d0
    do i=1,size(bodies)
       energy = energy + 0.5d0 * bodies(i)%mass *  &
            ( bodies(i)%vx * bodies(i)%vx + &
            bodies(i)%vy * bodies(i)%vy + &
            bodies(i)%vz * bodies(i)%vz)

       do j=i+1,size(bodies)
          dx = bodies(i)%x - bodies(j)%x
          dy = bodies(i)%y - bodies(j)%y
          dz = bodies(i)%z - bodies(j)%z
          distance = sqrt(dx*dx + dy*dy + dz*dz)
          energy = energy - (bodies(i)%mass * bodies(j)%mass) / distance;
       end do

    end do
  end function energy

end program nbody
