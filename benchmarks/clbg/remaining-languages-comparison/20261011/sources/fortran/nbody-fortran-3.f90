! The Computer Language Benchmarks Game
! https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
!
!   Translated from Mark C. Lewis nbody.java by Simon Geard
!   Revised by Mike Garrahan
!
! ifort -O3 -xHost -o nbody nbody.f90
!

program nbody
  implicit none
  integer, parameter :: dp = selected_real_kind(15, 307)

  real(dp), parameter :: TSTEP = 0.01d0
  real(dp), parameter :: PI = 3.141592653589793d0
  real(dp), parameter :: SOLAR_MASS = 4 * PI * PI
  real(dp), parameter :: DAYS_PER_YEAR = 365.24d0

  integer, parameter :: NB = 5
  integer, parameter :: NPAIR = NB * (NB - 1) / 2

  real(dp) :: x(3,NB), v(3,NB), mass(NB)
  real(dp) :: e

  integer :: nstep, i
  character(len=8) :: argv

  call getarg(1, argv)
  read (argv,*) nstep

  call init(x, v, mass)

  e = energy(x, v, mass)
  write (*,'(f12.9)') e

  do i = 1, nstep
     call advance(x, v, mass)
  end do

  e = energy(x, v, mass)
  write (*,'(f12.9)') e

contains

  subroutine advance(x, v, mass)
    real(dp), intent(inout) :: x(3,NB), v(3,NB)
    real(dp), intent(in) :: mass(NB)

    real(dp) :: r(3,NPAIR), rmag(3)
    real(dp) :: distance(NPAIR), mag(NPAIR)
    integer :: i, j, k

    k = 1
    do i = 1, NB - 1
       do j = i + 1, NB
          r(:,k) = x(:,i) - x(:,j)
          k = k + 1
       end do
    end do

    distance = sqrt(sum(r**2, dim=1))
    mag = TSTEP / distance**3

    k = 1
    do i = 1, NB - 1
       do j = i + 1, NB
          rmag = mag(k) * r(:,k)
          v(:,i) = v(:,i) - mass(j) * rmag
          v(:,j) = v(:,j) + mass(i) * rmag
          k = k + 1
       end do
    end do

    x = x + TSTEP * v
  end subroutine advance

  function energy(x, v, mass)
    real(dp) :: energy
    real(dp), intent(in) :: x(3,NB), v(3,NB), mass(NB)

    real(dp) :: r(3), distance, ke, pe
    integer :: i, j

    ke = 0.5d0 * dot_product(mass, sum(v**2, dim=1))
    pe = 0.0d0
    do i = 1, NB - 1
       do j = i + 1, NB
          r = x(:,i) - x(:,j)
          distance = sqrt(sum(r**2))
          pe = pe - mass(i) * mass(j) / distance
       end do
    end do
    energy = ke + pe
  end function energy

  subroutine init(x, v, mass)
    real(dp), intent(out) :: x(3,NB), v(3,NB), mass(NB)
    integer, parameter :: SUN = 1, JUPITER = 2, SATURN = 3, &
         URANUS = 4, NEPTUNE = 5
    real(dp) :: momentum(3)

    x(:,JUPITER) = (/ &
         4.84143144246472090d+00, &
        -1.16032004402742839d+00, &
        -1.03622044471123109d-01 /)
    v(:,JUPITER) = (/ &
         1.66007664274403694d-03, &
         7.69901118419740425d-03, &
        -6.90460016972063023d-05 /)
    mass(JUPITER) = &
         9.54791938424326609d-04

    x(:,SATURN) = (/ &
         8.34336671824457987d+00, &
         4.12479856412430479d+00, &
        -4.03523417114321381d-01 /)
    v(:,SATURN) = (/ &
        -2.76742510726862411d-03, &
         4.99852801234917238d-03, &
         2.30417297573763929d-05 /)
    mass(SATURN) = &
         2.85885980666130812d-04

    x(:,URANUS) = (/ &
         1.28943695621391310d+01, &
        -1.51111514016986312d+01, &
        -2.23307578892655734d-01 /)
    v(:,URANUS) = (/ &
         2.96460137564761618d-03, &
         2.37847173959480950d-03, &
        -2.96589568540237556d-05 /)
    mass(URANUS) = &
         4.36624404335156298d-05

    x(:,NEPTUNE) = (/ &
         1.53796971148509165d+01, &
        -2.59193146099879641d+01, &
         1.79258772950371181d-01 /)
    v(:,NEPTUNE) = (/ &
         2.68067772490389322d-03, &
         1.62824170038242295d-03, &
        -9.51592254519715870d-05 /)
    mass(NEPTUNE) = &
         5.15138902046611451d-05

    x(:,SUN) = 0.0d0
    v(:,SUN) = 0.0d0
    mass(SUN) = 1.0d0

    v = v * DAYS_PER_YEAR
    mass = mass * SOLAR_MASS

    momentum = matmul(v, mass)
    v(:,SUN) = v(:,SUN) - momentum / mass(SUN)
  end subroutine init

end program nbody
