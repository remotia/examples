#[cfg(feature = "xcap")]
pub mod xcap {
    use crate::data::Buffers;
    use platform_dependant_screen_snapper::xcap_capturer::{XCapCapturer, xcap_utils};

    const MONITOR_ID: usize = 0;

    pub fn fetch_screen_resolution() -> (u32, u32) {
        xcap_utils::display_size(MONITOR_ID)
    }

    pub fn capturer_processor() -> XCapCapturer<Buffers> {
        XCapCapturer::builder()
            .buffer_key(Buffers::CapturedScreenBuffer)
            .monitor_id(MONITOR_ID)
            .build()
    }
}

#[cfg(feature = "xcap")]
pub use xcap::*;

#[cfg(feature = "wayshot")]
pub mod libwayshot {
    use platform_dependant_screen_snapper::wayshot_capturer::{WayshotCapturer, wayshot_utils};

    use crate::data::Buffers;

    pub fn fetch_screen_resolution() -> (u32, u32) {
        wayshot_utils::display_size()
    }

    pub fn capturer_processor() -> WayshotCapturer<Buffers> {
        WayshotCapturer::builder()
            .buffer_key(Buffers::CapturedScreenBuffer)
            .build()
    }
}

#[cfg(feature = "wayshot")]
pub use libwayshot::*;
