use extism_pdk::{MemoryHandle, memory::internal::load};
use warpgate_api::SendRequestOutput;

/// Populate the body of a [`SendRequestOutput`] by loading the raw bytes
/// from WASM memory, using the body offset and length.
#[doc(hidden)]
pub fn populate_send_request_output(output: &mut SendRequestOutput) {
    if output.body.is_empty() {
        let handle = unsafe { MemoryHandle::new(output.body_offset, output.body_length) };
        let mut body = vec![0; handle.length as usize];

        load(handle, &mut body);

        output.body = body;
    }
}
