#[cfg(feature = "native_client")]
mod native;
#[cfg(feature = "native_client")]
pub use native::Client as NativeClient;

#[cfg(feature = "web_client")]
mod web;
#[cfg(feature = "web_client")]
pub use web::Client as WebClient;
