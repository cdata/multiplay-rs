use super::panic::set_panic_hook;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Client {}

#[wasm_bindgen]
impl Client {
    pub fn new(_url: &str) -> Self {
        set_panic_hook();
        Client {}
    }
}
