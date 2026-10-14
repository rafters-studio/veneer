//! Node binding for veneer. The `veneer` npm package loads this library.

use napi_derive::napi;

/// The version of the check this binding was built against.
#[napi]
pub fn check_version() -> String {
    veneer_check::version().to_string()
}
