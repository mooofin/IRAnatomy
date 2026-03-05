pub mod app;
pub mod components;
pub mod highlight;
pub mod server_functions;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "hydrate")] {
        use leptos::*;
        use wasm_bindgen::prelude::wasm_bindgen;
        use crate::app::*;

        #[wasm_bindgen]
        pub fn hydrate() {
            // initializes logging using the `console_error_panic_hook` crate
            _ = console_log::init_with_level(log::Level::Debug);
            console_error_panic_hook::set_once();
            
            leptos::mount_to_body(App);
        }
    }
}
