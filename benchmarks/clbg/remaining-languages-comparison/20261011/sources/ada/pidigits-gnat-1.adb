--  The Computer Language Benchmarks Game
--  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--  Calculate digits of pi using the
--  Unbounded Spigot Algorithms
--
--  From Pascal code by Vincent Snijders
--  gmp headers by Karl-Michael Schindler
--  Translated by (New) P2Ada v. 17-June-2006

with Ada.Text_IO;                       use Ada.Text_IO;
with Ada.Integer_Text_IO;               use Ada.Integer_Text_IO;
with Ada.Command_Line;                  use Ada.Command_Line;
with Ada.Calendar;                      use Ada.Calendar;

with Interfaces.C;

procedure pidigits is

  pragma Suppress(All_Checks);

  package GMP_mini is
    type mp_limb_t is new Interfaces.C.unsigned;
    type mp_ptr is access mp_limb_t;

    type mpz_t is record
      mp_alloc, mp_size : Interfaces.C.int;
      mp_d    : mp_ptr;
    end record;

    procedure mpz_init (Dest: out mpz_t);
    pragma Import(C, mpz_init, "__gmpz_init");

    procedure mpz_init_set_ui (Dest: out mpz_t; Src: in Interfaces.C.unsigned_long);
    pragma Import(C, mpz_init_set_ui, "__gmpz_init_set_ui");

    procedure mpz_mul_ui (Dest: out mpz_t; Src1: in mpz_t; Src2: in Interfaces.C.unsigned_long);
    pragma Import(C, mpz_mul_ui, "__gmpz_mul_ui");

    procedure mpz_mul_si (Dest: out mpz_t; Src1: in mpz_t; Src2: in Interfaces.C.int);
    pragma Import(C, mpz_mul_si, "__gmpz_mul_si");

    procedure mpz_add (Dest: out mpz_t; Src1, Src2: in mpz_t);
    pragma Import(C, mpz_add, "__gmpz_add");

    procedure mpz_tdiv_q (Dest: out mpz_t; Src1, Src2: in mpz_t);
    pragma Import(C, mpz_tdiv_q, "__gmpz_tdiv_q");

    function  mpz_get_ui (Src: in mpz_t) return Interfaces.C.unsigned_long;
    pragma Import(C, mpz_get_ui, "__gmpz_get_ui");

    pragma Linker_Options("-lgmp");

  end GMP_Mini;

  procedure Print_pi_digits(num_digits: Integer) is
    use GMP_mini;

    q,r,s,t: mpz_t; --  Transformation matrix components

    u,v,w: mpz_t;   --  Temporary variables

    k,digit: Interfaces.C.int;
    c,i: Integer;
    line: String(1 ..10);

    function Extract(x: Interfaces.C.Unsigned_long) return Interfaces.C.int is
    begin
      mpz_mul_ui(u, q, x);
      mpz_add(u, u, r);
      mpz_mul_ui(v, s, x);
      mpz_add(v, v, t);
      mpz_tdiv_q(w, u, v);
      return Interfaces.C.int(mpz_get_ui(w));
    end Extract;

    use Interfaces.C;

    function IsSafe return Boolean is
    begin
      return digit = Extract(4);
    end IsSafe;

    procedure Produce is
    begin
      mpz_mul_si(r, r, 10);
      mpz_mul_si(v, t, -10 * digit);
      mpz_add(r, r, v);
      mpz_mul_si(q, q, 10);
    end Produce;

    procedure Consume is
    begin
      k:= k + 1;
      mpz_mul_si(r, r, 2*k+1);
      mpz_mul_si(u, q, 4*k+2);
      mpz_add(r, r, u);
      mpz_mul_si(t, t, 2*k+1);
      mpz_mul_si(v, s, 4*k+2);
      mpz_add(t, t, v);
      mpz_mul_si(s, s, k);
      mpz_mul_si(q, q, k);
    end Consume;

  begin
    k := 0;
    i := 0;
    c := 0;
    mpz_init_set_ui(q, 1);
    mpz_init_set_ui(r, 0);
    mpz_init_set_ui(s, 0);
    mpz_init_set_ui(t, 1);
    mpz_init(u);
    mpz_init(v);
    mpz_init(w);
    while i < num_digits loop
      digit := Extract(3);
      while not IsSafe loop
        Consume;
        digit:= Extract(3);
      end loop;
      Produce;
      c:= c + 1;
      line(c) := Character'Val(Character'Pos('0')+digit);
      i:= i + 1;
      if c = 10 then
        Put(line & ASCII.HT & ':');
        Put(i,0); New_Line;
        c := 0;
      end if;
    end loop;
    if  c/=0 then
      Put(line(1..c));
      for i in c+1..10 loop
        Put(' ');
      end loop;
      Put(ASCII.HT & ':');
      Put(i,0);
      New_Line;
    end if;
  end Print_pi_digits;

  n: Integer;

  t0,t1: Time;
  timing: constant Boolean:= False;

begin
  n:= 2_500;
  if Argument_Count=1 then
    n:= Integer'Value(Argument(1));
  end if;
  if timing then
    t0:= Clock;
  end if;
  Print_pi_digits(n);
  if timing then
    t1:= Clock;
    Put("Time in seconds: " & Duration'Image(t1-t0) & " [press return]");
    Skip_Line;
  end if;
end pidigits;
