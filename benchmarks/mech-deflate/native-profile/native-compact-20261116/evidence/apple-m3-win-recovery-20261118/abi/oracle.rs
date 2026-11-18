use core::ffi::c_void;
use mech_gpu::native_artifact::compact_runtime::{Heap,INVALID};
#[repr(C)] struct Pair { data:u64, owner_or_status:u64 }
type Entry=unsafe extern "C" fn(*mut c_void,u64)->Pair;
unsafe extern "C" {
 fn mech_compact_allocate_backing(h:*mut c_void,n:u64,s:u64,a:u64,z:u64)->Pair;
 fn mech_compact_free(h:*mut c_void,o:*mut core::ffi::c_void)->u64;
 fn oracle_1_0(h:*mut c_void,n:u64)->Pair; fn oracle_1_1(h:*mut c_void,n:u64)->Pair;
 fn oracle_2_0(h:*mut c_void,n:u64)->Pair; fn oracle_2_1(h:*mut c_void,n:u64)->Pair;
 fn oracle_4_0(h:*mut c_void,n:u64)->Pair; fn oracle_4_1(h:*mut c_void,n:u64)->Pair;
 fn oracle_8_0(h:*mut c_void,n:u64)->Pair; fn oracle_8_1(h:*mut c_void,n:u64)->Pair;
 fn oracle_duplicate(h:*mut c_void,n:u64)->Pair;fn oracle_true_bits(h:*mut c_void,n:u64)->Pair;
 fn oracle_align(h:*mut c_void,n:u64)->Pair;fn oracle_stride(h:*mut c_void,n:u64)->Pair;
 fn oracle_bad_align(h:*mut c_void,n:u64)->Pair;fn oracle_other_provider(h:*mut c_void,n:u64)->Pair;
 fn oracle_dynamic(h:*mut c_void,n:u64,s:u64,a:u64,z:u64)->Pair;
}
#[unsafe(no_mangle)] unsafe extern "C" fn oracle_alternate_allocate(h:*mut c_void,n:u64,s:u64,a:u64,z:u64)->Pair {
 unsafe { mech_compact_allocate_backing(h,n,s,a,z) }
}
fn main() {
 let entries:[(Entry,u64,u64,u64);14]=[
  (oracle_1_0,1,1,0),(oracle_1_1,1,1,1),(oracle_2_0,2,2,0),(oracle_2_1,2,2,1),
  (oracle_4_0,4,4,0),(oracle_4_1,4,4,1),(oracle_8_0,8,8,0),(oracle_8_1,8,8,1),
  (oracle_duplicate,4,4,1),(oracle_true_bits,4,4,u64::MAX),(oracle_align,4,8,1),
  (oracle_stride,3,1,1),(oracle_bad_align,4,3,1),(oracle_other_provider,4,4,1)];
 let mut checks=0;
 for (entry,stride,align,zero) in entries {
  let missing=unsafe { entry(std::ptr::null_mut(),1) };assert_eq!((missing.data,missing.owner_or_status),(0,INVALID));checks+=1;
  for capacity in [0,1,17,2048,32768,u64::MAX] {
   let mut before=Heap::new();let mut after=Heap::new();
   let original=unsafe { mech_compact_allocate_backing((&mut before as *mut Heap).cast(),capacity,stride,align,zero) };
   let result=unsafe { entry((&mut after as *mut Heap).cast(),capacity) };
   assert_eq!(original.data==0,result.data==0);
   if original.data==0 {assert_eq!(original.owner_or_status,result.owner_or_status);}
   else if capacity==0 {assert_eq!((original.data,original.owner_or_status),(result.data,result.owner_or_status));}
   else {
    assert_eq!(result.data%align,0);
    if zero!=0 {assert!(unsafe{std::slice::from_raw_parts(result.data as *const u8,(capacity*stride) as usize)}.iter().all(|&x|x==0));}
    assert_eq!(unsafe { mech_compact_free((&mut before as *mut Heap).cast(),original.owner_or_status as *mut _) },0);
    assert_eq!(unsafe { mech_compact_free((&mut after as *mut Heap).cast(),result.owner_or_status as *mut _) },0);
   }
   assert_eq!(before.counters.payload_requests,after.counters.payload_requests);
   assert_eq!(before.counters.requested_payload_bytes,after.counters.requested_payload_bytes);
   assert_eq!(before.counters.zeroed_bytes,after.counters.zeroed_bytes);
   assert_eq!(before.counters.live_payload_bytes,after.counters.live_payload_bytes);
   assert_eq!(before.counters.peak_payload_bytes,after.counters.peak_payload_bytes);
   assert_eq!(after.counters.live_payload_bytes,0);checks+=1;
  }
 }
 let mut heap=Heap::new();let dynamic=unsafe { oracle_dynamic((&mut heap as *mut Heap).cast(),17,4,8,1) };assert_ne!(dynamic.data,0);assert_eq!(unsafe{mech_compact_free((&mut heap as *mut Heap).cast(),dynamic.owner_or_status as *mut _)},0);checks+=1;
 println!("Passed {checks} real native ABI, initialization, overflow, null-context, dynamic-geometry and physical cleanup checks");
}
