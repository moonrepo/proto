use proto_pdk_api::{
    BuildInstructionsInput, DownloadPrebuiltInput, NativeInstallInput, VirtualPath,
};
use serde::{Serialize, de::DeserializeOwned};

// Older proto versions don't pass `temp_dir`, so it must not be required
fn deserialize_without_temp_dir<T: Default + Serialize + DeserializeOwned>() -> T {
    let mut value = serde_json::to_value(T::default()).unwrap();

    value
        .as_object_mut()
        .unwrap()
        .remove("temp_dir")
        .expect("temp_dir field");

    serde_json::from_value(value).unwrap()
}

mod temp_dir {
    use super::*;

    #[test]
    fn defaults_for_build_instructions() {
        let input: BuildInstructionsInput = deserialize_without_temp_dir();

        assert_eq!(input.temp_dir, VirtualPath::default());
    }

    #[test]
    fn defaults_for_download_prebuilt() {
        let input: DownloadPrebuiltInput = deserialize_without_temp_dir();

        assert_eq!(input.temp_dir, VirtualPath::default());
    }

    #[test]
    fn defaults_for_native_install() {
        let input: NativeInstallInput = deserialize_without_temp_dir();

        assert_eq!(input.temp_dir, VirtualPath::default());
    }
}
