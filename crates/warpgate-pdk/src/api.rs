use extism_pdk::{memory::internal::memory_bytes, MemoryHandle};
use warpgate_api::SendRequestOutput;

pub fn populate_send_request_output(output: &mut SendRequestOutput) {
    if output.body.is_empty() {
        output.body =
            unsafe { memory_bytes(MemoryHandle::new(output.body_offset, output.body_length)) };
    }
}
