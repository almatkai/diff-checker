//! Test-only registration/installation stubs around BYTE-EXACT beta native sessions.
#[rustfmt::skip]
#[path = "../beta/src-tauri/src/native_core/common.rs"]
pub mod common;
#[path = "extensions.rs"]
pub mod extensions;
#[path = "../../src/tezbar_adapter.rs"]
pub mod production_adapter;
#[rustfmt::skip]
#[path = "../beta/src-tauri/src/native_core/extensions/v2_sdk.rs"]
pub mod v2_sdk;
pub mod native_core {
    pub use crate::common;
    pub use crate::extensions;
}
