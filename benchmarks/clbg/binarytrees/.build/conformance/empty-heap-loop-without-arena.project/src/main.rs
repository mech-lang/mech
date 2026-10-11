unsafe extern "C" { fn mech_control_entry(arguments:*const u64,output:*mut u64)->u64; }
fn main(){
    // SAFETY: this application links the object and descriptor emitted together
    // by the admitted native control compiler; the ABI is library-owned.
    if let Err(error)=unsafe{mech_gpu::control::run_cli(mech_control_entry,include_str!("../control.json"))}{eprintln!("{error}");std::process::exit(1);}
}
