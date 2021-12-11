#[macro_use]
extern crate log;

mod client;
mod utils;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// #[wasm_bindgen]
// extern "C" {
//     fn alert(s: &str);
// }

// #[wasm_bindgen]
// pub fn greet() {
//     alert("Hello, client!");
// }

#[wasm_bindgen]
pub struct MultiplayrsClient {}

#[wasm_bindgen]
impl MultiplayrsClient {
    pub fn new(_url: &str) -> Self {
        MultiplayrsClient {}
    }
}
