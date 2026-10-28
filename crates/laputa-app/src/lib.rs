//! Laputa application-level presentation state.
//!
//! Product UI state lives here so it can be tested without starting a native GPUI renderer.

pub mod desktop_state;
pub mod preview_controller;
#[cfg(feature = "desktop-ui")]
pub mod preview_frame_image;
pub mod preview_playback;
pub mod preview_player;
pub mod preview_request;
pub mod project_session;
pub mod workspace_state;
