#[cfg(not(any(feature = "xcap", feature = "wayshot")))]
compile_error!("No snapper backened enabled");

#[cfg(all(feature = "xcap", feature = "wayshot"))]
compile_error!("Compiling with both wayshot and xcap support is not currently supported.");

pub mod png_saver;

#[cfg(feature = "wayshot")]
pub mod wayshot_capturer;

#[cfg(feature = "xcap")]
pub mod xcap_capturer;
