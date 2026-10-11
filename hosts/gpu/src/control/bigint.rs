//! Entry-owned mutable cells and frame-owned bigint operator values. Handles
//! are typed by the IR, generation checked, and unavailable to workers.
use rug::{Assign, Integer, ops::{DivRounding, SubFrom}};
use std::cell::RefCell;
use std::collections::BTreeSet;

const VALUE_TAG: u64 = 1 << 63;
const GENERATION_MAX: u32 = 0x7fff_ffff;

#[derive(Default)]
struct ValueSlot {
    generation: u32,
    live: bool,
}

#[derive(Default)]
struct Session {
    values: Vec<Integer>,
    quotient: Integer,
    error: Option<String>,
    power2_shifts: bool,
    // Pinned named cells retain their old one-based handles. Pure operator
    // values use generation-tagged handles and recyclable limb storage.
    slots: Vec<ValueSlot>,
    free: Vec<usize>,
    frames: Vec<Vec<usize>>,
    spare_frames: Vec<Vec<usize>>,
}
impl Session {
    fn index(&self, handle: u64) -> Result<usize, String> {
        let tagged = handle & VALUE_TAG != 0;
        let number = if tagged { handle & u32::MAX as u64 } else { handle };
        let index = usize::try_from(number).ok().and_then(|n| n.checked_sub(1))
            .filter(|&n| n < self.values.len()).ok_or("invalid big-integer handle")?;
        let slot = &self.slots[index];
        let generation = ((handle >> 32) & GENERATION_MAX as u64) as u32;
        if (tagged && (!slot.live || slot.generation != generation))
            || (!tagged && slot.generation != 0)
        {
            return Err("retired or invalid big-integer handle".into());
        }
        Ok(index)
    }
    fn frame_enter(&mut self) {
        if self.frames.is_empty() { self.frames.push(Vec::new()); }
        self.frames.push(self.spare_frames.pop().unwrap_or_default());
    }
    fn frame_roots(&mut self, roots: &[u64], close: bool) -> Result<(), String> {
        let roots = roots.iter().map(|&h| self.index(h)).collect::<Result<BTreeSet<_>, _>>()?;
        if self.frames.is_empty() || (close && self.frames.len() < 2) {
            return Err("unbalanced big-integer value frame".into());
        }
        let mut owned = if close { self.frames.pop().unwrap() }
            else { std::mem::take(self.frames.last_mut().unwrap()) };
        owned.retain(|&index| {
            if roots.contains(&index) { true }
            else { self.slots[index].live = false; self.free.push(index); false }
        });
        // Only this frame's allocations move to the caller. Borrowed inputs
        // never change owner, even when returned multiple times or unchanged.
        if close {
            self.frames.last_mut().unwrap().extend(owned.drain(..));
            self.spare_frames.push(owned);
        } else { *self.frames.last_mut().unwrap() = owned; }
        Ok(())
    }
    fn value_slot(&mut self) -> Result<(usize, u64), String> {
        if self.frames.is_empty() { self.frames.push(Vec::new()); }
        let index = if let Some(index) = self.free.pop() {
            self.slots[index].generation = self.slots[index].generation.checked_add(1)
                .filter(|&g| g <= GENERATION_MAX).ok_or("big-integer generation exhausted")?;
            self.slots[index].live = true;
            index
        } else {
            if self.values.len() >= u32::MAX as usize { return Err("big-integer storage exhausted".into()); }
            self.values.push(Integer::new());
            self.slots.push(ValueSlot { generation: 1, live: true });
            self.values.len() - 1
        };
        self.frames.last_mut().unwrap().push(index);
        Ok((index, VALUE_TAG | (u64::from(self.slots[index].generation) << 32) | (index as u64 + 1)))
    }
    fn pair(&mut self, dst: usize, src: usize) -> (&mut Integer, &Integer) {
        assert_ne!(dst, src);
        if dst < src {
            let (left, right) = self.values.split_at_mut(src);
            (&mut left[dst], &right[0])
        } else {
            let (left, right) = self.values.split_at_mut(dst);
            (&mut right[0], &left[src])
        }
    }
    fn operation(&mut self, action: u64, a: u64, b: u64, c: u64) -> Result<u64, String> {
        if action == 10 {
            self.values.push(Integer::from(a));
            self.slots.push(ValueSlot::default());
            return Ok(self.values.len() as u64);
        }
        if action == 19 {
            let (index, handle) = self.value_slot()?;
            self.values[index].assign(a);
            return Ok(handle);
        }
        if action == 40 { self.frame_enter(); return Ok(0); }
        if action == 32 {
            return self.values[self.index(a)?].to_u64().ok_or_else(|| "big integer does not fit u64".into());
        }
        if action == 25 {
            let x = self.index(a)?;
            let (index, handle) = self.value_slot()?;
            let (d, x) = self.pair(index, x);
            d.assign(-x);
            return Ok(handle);
        }
        if matches!(action, 11..=14 | 16 | 17) && a & VALUE_TAG != 0 {
            return Err("big-integer operator values are immutable; mutable destination must be a named cell".into());
        }
        let dst = self.index(a)?;
        let src = self.index(b)?;
        if (26..=31).contains(&action) {
            let ordering = self.values[dst].cmp(&self.values[src]);
            return Ok(u64::from(match action {
                26 => ordering.is_eq(), 27 => !ordering.is_eq(),
                28 => ordering.is_lt(), 29 => !ordering.is_gt(),
                30 => ordering.is_gt(), 31 => !ordering.is_lt(), _ => unreachable!(),
            }));
        }
        if matches!(action, 20..=24 | 33) {
            if matches!(action, 23 | 24 | 33) && self.values[src] == 0 {
                return Err("big-integer division by zero".into());
            }
            let (index, handle) = self.value_slot()?;
            let mut result = std::mem::take(&mut self.values[index]);
            // Each destination is distinct and retains its limb capacity.
            // Borrowed incomplete GMP expressions avoid a fresh bigint copy
            // on every operator application and every loop iteration.
            match action {
                20 => result.assign(&self.values[dst] + &self.values[src]),
                21 => result.assign(&self.values[dst] - &self.values[src]),
                22 => result.assign(&self.values[dst] * &self.values[src]),
                23 => result.assign(&self.values[dst] / &self.values[src]),
                24 => result.assign(&self.values[dst] % &self.values[src]),
                33 => result.assign((&self.values[dst]).div_floor(&self.values[src])),
                _ => unreachable!(),
            }
            self.values[index] = result;
            return Ok(handle);
        }
        if action == 18 { return Ok(u64::from(self.values[dst] <= self.values[src])); }
        if action == 15 {
            if self.values[src] == 0 { return Err("big-integer division by zero".into()); }
            // Emit a nonaliasing division into the reusable quotient. Copying
            // the numerator first would force GMP's alias-safe temporary path.
            // For equal operand signs, truncation is exactly floor. GMP's
            // general floor routine also computes a remainder; quotient-only
            // division avoids that work without changing signed semantics.
            if (self.values[dst] < 0) == (self.values[src] < 0) {
                self.quotient.assign(&self.values[dst] / &self.values[src]);
            } else {
                self.quotient.assign((&self.values[dst]).div_floor(&self.values[src]));
            }
            return self.quotient.to_u64().ok_or_else(|| "quotient does not fit u64".into());
        }
        match action {
            11 if dst != src => { let (d, s) = self.pair(dst, src); d.assign(s); }
            11 => {}
            12 if dst == src => {
                if self.power2_shifts && c.is_power_of_two() { self.values[dst] <<= c.trailing_zeros(); }
                else { self.values[dst] *= c; }
            }
            12 => {
                let shift = self.power2_shifts && c.is_power_of_two();
                let (d, s) = self.pair(dst, src);
                if shift { d.assign(s << c.trailing_zeros()); }
                else { d.assign(s * c); }
            }
            13 | 14 => {
                let rhs = self.index(c)?;
                if dst == src && dst == rhs {
                    if action == 13 { self.values[dst] *= 2u32; }
                    else { self.values[dst].assign(0); }
                } else if dst == src {
                    let (d, r) = self.pair(dst, rhs);
                    if action == 13 { *d += r; } else { *d -= r; }
                } else if dst == rhs {
                    let (d, s) = self.pair(dst, src);
                    if action == 13 { *d += s; } else { d.sub_from(s); }
                } else {
                    // Take only the destination so sources stay borrowed and
                    // the destination's limb capacity is reused without copies.
                    let mut d = std::mem::take(&mut self.values[dst]);
                    if action == 13 { d.assign(&self.values[src] + &self.values[rhs]); }
                    else { d.assign(&self.values[src] - &self.values[rhs]); }
                    self.values[dst] = d;
                }
            }
            16 | 17 if dst == src => {
                // Aliasing has ordinary mathematical semantics, including
                // the signed result of x - x*c; no wrapping scalar factor.
                let factor = if action == 16 { Integer::from(c) + 1 }
                    else { Integer::from(1) - c };
                self.values[dst] *= factor;
            }
            16 | 17 => {
                let (d, s) = self.pair(dst, src);
                if action == 16 { *d += s * c; } else { *d -= s * c; }
            }
            _ => return Err("unknown big-integer operation".into()),
        }
        Ok(a)
    }
}
thread_local! { static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) }; }

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) { SESSION.with(|s| { s.borrow_mut().take(); }); }
}
pub(super) fn with_session<T>(run: impl FnOnce() -> T) -> Result<T, String> {
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        if s.is_some() { return Err("nested big-integer sessions unsupported".to_owned()); }
        *s = Some(Session { power2_shifts: std::env::var("MECH_BIGINT_POWER2_SHIFTS").as_deref() == Ok("1"), ..Session::default() });
        Ok(())
    })?;
    let _guard = Guard;
    let result = run();
    let error = SESSION.with(|s| s.borrow_mut().as_mut().and_then(|s| s.error.take()));
    if let Some(error) = error { Err(error) } else { Ok(result) }
}

/// Apply one typed general arithmetic operation, never a source algorithm.
/// # Safety
/// The admitted same-object call supplies exclusive, initialized u64 output
/// storage that remains live for this synchronous operation and does not escape.
/// For frame actions 41/42, a also addresses b (at most 4096) initialized,
/// aligned u64 handle lanes in a live compiler stack slot. The slot remains
/// non-null and aligned even for zero lanes; these roots are borrowed only
/// during this synchronous call. No source program can construct these actions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_control_bigint(action: u64, a: u64, b: u64, c: u64, output: *mut u64) -> u64 {
    SESSION.with(|s| {
        let mut s = s.borrow_mut();
        let Some(s) = s.as_mut() else { return 10; };
        let result = if matches!(action, 41 | 42) {
            let count = match usize::try_from(b) {
                Ok(count) if count <= 4096 => count,
                _ => { s.error = Some("invalid big-integer frame root count".into()); return 10; }
            };
            // SAFETY: frame actions are emitted only by the typed compiler;
            // a points to count initialized u64 lanes in its live stack slot.
            let roots = unsafe { std::slice::from_raw_parts(a as *const u64, count) };
            s.frame_roots(roots, action == 41).map(|()| 0)
        } else { s.operation(action, a, b, c) };
        match result {
            Ok(value) => {
                // SAFETY: the emitter supplies its exclusive live u64 slot.
                unsafe { *output = value; }
                0
            }
            Err(error) => { s.error = Some(error); 10 }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_arithmetic_aliasing_and_reused_storage() {
        let mut s = Session::default();
        let x = s.operation(10, 7, 0, 0).unwrap();
        let y = s.operation(10, 3, 0, 0).unwrap();
        let z = s.operation(10, 0, 0, 0).unwrap();
        for (op,a,b,c,want) in [(12,z,x,11,77), (13,z,x,y,10), (14,z,y,x,-4),
            (14,y,x,y,4), (16,x,y,5,27), (17,x,x,2,-27), (13,y,y,y,8), (14,y,y,y,0)] {
            s.operation(op,a,b,c).unwrap();
            assert_eq!(s.values[(a-1) as usize], want);
        }
        s.operation(11,z,x,0).unwrap();
        assert_eq!(s.values[(z-1) as usize], -27);
        s.operation(12,z,z,u64::MAX).unwrap();
        assert!(s.values[(z-1) as usize].significant_bits() > 64);
        assert!(s.operation(15,x,y,0).unwrap_err().contains("zero"));
        assert!(s.operation(11,0,x,0).is_err());
        s.power2_shifts = true;
        assert_eq!(s.operation(18,x,y,0).unwrap(),1);
        s.operation(12,z,x,4).unwrap();
        assert_eq!(s.values[(z-1) as usize],-108);
        s.operation(12,z,z,8).unwrap();
        assert_eq!(s.values[(z-1) as usize],-864);
    }
    #[test]
    fn floor_division_and_session_retirement() {
        let mut s = Session::default();
        let a = s.operation(10,7,0,0).unwrap();
        let b = s.operation(10,3,0,0).unwrap();
        assert_eq!(s.operation(15,a,b,0).unwrap(),2);
        s.values[0] = Integer::from(-7);
        s.values[1] = Integer::from(-3);
        assert_eq!(s.operation(15,a,b,0).unwrap(),2);
        s.values[1] = Integer::from(3);
        assert!(s.operation(15,a,b,0).is_err());
        assert_eq!(s.quotient,-3);
        for _ in 0..2 { with_session(|| {
            SESSION.with(|s| assert!(s.borrow().as_ref().unwrap().values.is_empty()));
            assert!(with_session(|| ()).is_err());
        }).unwrap(); }
        assert!(SESSION.with(|s| s.borrow().is_none()));
    }
    #[test]
    fn ordinary_operators_preserve_operands_and_signed_rounding() {
        let mut s = Session::default();
        s.frame_enter();
        let a = s.operation(10,7,0,0).unwrap();
        let b = s.operation(10,3,0,0).unwrap();
        let sum = s.operation(20,a,b,0).unwrap();
        let product = s.operation(22,a,b,0).unwrap();
        let negative = s.operation(25,a,0,0).unwrap();
        assert_eq!(s.operation(32,sum,0,0).unwrap(),10);
        assert_eq!(s.operation(32,product,0,0).unwrap(),21);
        assert_eq!(s.operation(32,a,0,0).unwrap(),7);
        assert_eq!(s.operation(32,b,0,0).unwrap(),3);
        for (action,want) in [(23,-2),(24,-1),(33,-3)] {
            let result = s.operation(action,negative,b,0).unwrap();
            assert_eq!(s.values[s.index(result).unwrap()],want);
        }
        let minus_b = s.operation(25,b,0,0).unwrap();
        let positive = s.operation(23,negative,minus_b,0).unwrap();
        assert_eq!(s.operation(32,positive,0,0).unwrap(),2);
        for (action,want) in [(26,0),(27,1),(28,0),(29,0),(30,1),(31,1)] {
            assert_eq!(s.operation(action,a,b,0).unwrap(),want);
        }
        assert!(s.operation(32,negative,0,0).unwrap_err().contains("fit u64"));
        let large = s.operation(19,u64::MAX,0,0).unwrap();
        let overflow = s.operation(22,large,large,0).unwrap();
        assert!(s.operation(32,overflow,0,0).unwrap_err().contains("fit u64"));
        let zero = s.operation(19,0,0,0).unwrap();
        for action in [23,24,33] {
            assert!(s.operation(action,a,zero,0).unwrap_err().contains("zero"));
        }
        s.frame_roots(&[],true).unwrap();
        assert!(s.index(sum).is_err());
        assert_eq!(s.operation(32,a,0,0).unwrap(),7);
    }
    #[test]
    fn value_frames_bound_loop_storage_and_preserve_borrowed_aliases() {
        let mut s = Session::default();
        s.frame_enter(); // entry
        let initial = s.operation(10,1,0,0).unwrap();
        let one = s.operation(19,1,0,0).unwrap();
        s.frame_enter(); // loop
        let mut a = initial;
        let mut b = initial;
        for _ in 0..25 {
            s.frame_enter(); // step's borrowed parameters
            let next = s.operation(20,a,b,0).unwrap();
            s.frame_roots(&[b,next,next],true).unwrap();
            a = b;
            b = next;
            s.frame_roots(&[a,b],false).unwrap();
        }
        assert_eq!(s.operation(32,a,0,0).unwrap(),121393);
        assert_eq!(s.operation(32,b,0,0).unwrap(),196418);
        assert_eq!(s.operation(32,initial,0,0).unwrap(),1);
        s.frame_roots(&[a,b],true).unwrap();
        assert!(s.values.len() <= 5,"loop leaked temporary cells");
        s.frame_enter();
        let mut counter = one;
        let mut retired = 0;
        for _ in 0..100_000 {
            s.frame_enter();
            let next = s.operation(20,counter,one,0).unwrap();
            s.frame_roots(&[next],true).unwrap();
            if counter != one { retired = counter; }
            counter = next;
            s.frame_roots(&[counter],false).unwrap();
        }
        assert_eq!(s.operation(32,counter,0,0).unwrap(),100001);
        assert!(s.index(retired).is_err(),"stale generation accepted");
        s.frame_roots(&[counter],true).unwrap();
        assert!(s.values.len() <= 7,"loop storage grew with iteration count");
        s.frame_roots(&[],true).unwrap();
        assert!(s.index(counter).is_err());
        assert_eq!(s.operation(32,initial,0,0).unwrap(),1);
    }
    #[test]
    fn value_results_can_be_borrowed_but_not_mutated_by_cell_apis() {
        let mut s = Session::default();
        s.frame_enter();
        let pinned = s.operation(10,7,0,0).unwrap();
        let source = s.operation(19,3,0,0).unwrap();
        let value = s.operation(20,pinned,source,0).unwrap();
        for (action,b,c) in [(11,pinned,0),(12,pinned,2),(13,pinned,source),
            (14,pinned,source),(16,pinned,2),(17,pinned,2)] {
            assert!(s.operation(action,value,b,c).unwrap_err().contains("immutable"));
        }
        assert_eq!(s.operation(32,value,0,0).unwrap(),10);
        s.operation(11,pinned,value,0).unwrap();
        assert_eq!(s.operation(32,pinned,0,0).unwrap(),10);
        assert_eq!(s.operation(32,value,0,0).unwrap(),10);
        s.frame_roots(&[],true).unwrap();
        assert_eq!(s.operation(32,pinned,0,0).unwrap(),10);
    }
    #[test]
    fn checked_value_error_retires_session_before_next_execution() {
        fn abi(action:u64,a:u64,b:u64) -> (u64,u64) {
            let mut output = 0;
            // SAFETY: a/b are scalar operands here, never root pointers, and
            // output is an exclusive live initialized u64 for the entire call.
            let status = unsafe { mech_control_bigint(action,a,b,0,&mut output) };
            (status,output)
        }
        let failure = with_session(|| {
            assert_eq!(abi(40,0,0).0,0);
            let (_,large) = abi(19,u64::MAX,0);
            let (_,overflow) = abi(22,large,large);
            assert_eq!(abi(32,overflow,0).0,10);
        });
        assert!(failure.unwrap_err().contains("fit u64"));
        let success = with_session(|| {
            SESSION.with(|s| {
                let s = s.borrow();
                let s = s.as_ref().unwrap();
                assert!(s.values.is_empty() && s.slots.is_empty() && s.free.is_empty()
                    && s.frames.is_empty() && s.spare_frames.is_empty());
            });
            assert_eq!(abi(40,0,0).0,0);
            let (_,a) = abi(19,9,0);
            let (_,b) = abi(19,2,0);
            let (_,sum) = abi(20,a,b);
            let (status,result) = abi(32,sum,0);
            assert_eq!(status,0);
            result
        }).unwrap();
        assert_eq!(success,11);
        assert!(SESSION.with(|s| s.borrow().is_none()));
    }
}

pub(super) fn apply(action: u64, a: u64, b: u64, c: u64) -> Result<u64, String> {
    SESSION.with(|s| s.borrow_mut().as_mut().ok_or("missing big-integer session".to_owned())?.operation(action, a, b, c))
}

pub(super) fn frame_roots(roots: &[u64], close: bool) -> Result<(), String> {
    SESSION.with(|s| s.borrow_mut().as_mut().ok_or("missing big-integer session".to_owned())?.frame_roots(roots, close))
}
