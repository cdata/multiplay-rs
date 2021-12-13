#[macro_use]
extern crate log;

// #[cfg(feature = "web_client")]
// use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub mod assets;
pub mod rtc;

#[cfg(test)]
mod test;
