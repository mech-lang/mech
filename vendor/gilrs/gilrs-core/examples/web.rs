#[cfg(target_arch = "wasm32")]
use gilrs_core::Gilrs;

// just to get around the wasm-bindgen crate conflicting with the wasm-bindgen feature
#[cfg(all(target_arch = "wasm32", not(cargo_web)))]
use wasm_bindgen_rs as wasm_bindgen;

fn main() {
    #[cfg(target_arch = "wasm32")]
    gamepad_loop(Gilrs::new().unwrap());
}

#[cfg(target_arch = "wasm32")]
fn gamepad_loop(mut gilrs: Gilrs) {
    use stdweb::web::set_timeout;

    while let Some(ev) = gilrs.next_event() {
        stdweb::console!(log, format!("{:?}", ev));
    }

    set_timeout(move || gamepad_loop(gilrs), 50);
}
