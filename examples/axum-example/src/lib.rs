pub mod app;
pub mod fileserv;
use crate::app::*;


use leptos::*;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::app::*;


#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    // initializes logging using the `log` crate
    // _ = console_log::init_with_level(log::Level::Info);
    console_error_panic_hook::set_once();

use leptos::logging::*;
log!("log Hello from Leptos WASM!");
//info!("inf Hello from Leptos WASM!");
warn!("war  sssssssssssssssssssss");
error!("err xxxxxxxxxxxxxxxxxxxxx");

    leptos::mount::hydrate_body(App);
}


