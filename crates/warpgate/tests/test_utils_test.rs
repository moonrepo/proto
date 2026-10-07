use std::path::Path;
use warpgate::test_utils::ConfigBuilder;
use warpgate_api::{HostArch, HostEnvironment, HostLibc, HostOS, HostPlatform};

fn get_host_environment(builder: ConfigBuilder) -> HostEnvironment {
    serde_json::from_str(builder.build().get("host_environment").unwrap()).unwrap()
}

mod config_builder {
    use super::*;

    #[test]
    fn sets_host_from_platform() {
        let mut builder = ConfigBuilder::new(Path::new("/sandbox"), Path::new("/home"));
        builder.host(HostPlatform::parse("arm64-linux-musl").unwrap());

        let env = get_host_environment(builder);

        assert_eq!(env.os, HostOS::Linux);
        assert_eq!(env.arch, HostArch::Arm64);
        assert_eq!(env.libc, HostLibc::Musl);
    }

    #[test]
    fn defaults_host_to_current_platform() {
        let builder = ConfigBuilder::new(Path::new("/sandbox"), Path::new("/home"));
        let platform = HostPlatform::from_env();

        let env = get_host_environment(builder);

        assert_eq!(env.os, platform.os);
        assert_eq!(env.arch, platform.arch);
        assert_eq!(env.libc, platform.libc);
    }
}
