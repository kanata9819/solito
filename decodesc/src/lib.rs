//! Decode VTE callback data into typed CSI, OSC, and ESC messages.
//!
//! Call the decoder matching the callback you already received; no generic
//! event wrapper is needed.

mod csi;
mod esc;
mod osc;

pub use csi::{CsiMessage, EraseMode, TabClearMode, decode_csi};
pub use esc::{EscMessage, decode_esc};
pub use osc::{OscMessage, decode_osc};
