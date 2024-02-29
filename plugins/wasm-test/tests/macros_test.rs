use proto_pdk_test_utils::*;

// This just tests that the macro generated code is correct,
// and doesn't actually pass.

generate_download_install_tests!("wasm-test", "1.2.3");

generate_resolve_versions_tests!("wasm-test", {
    "latest" => "1.2.3",
});

generate_shims_test!("wasm-test", ["other"]);
