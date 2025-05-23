use warpgate_api::VirtualPath;

mod virtual_path {
    use super::*;

    mod parent_real {
        use super::*;

        #[test]
        fn returns_some_parent() {
            let vp = VirtualPath::Real("/some/absolute/path".into());

            assert_eq!(
                vp.parent().unwrap(),
                VirtualPath::Real("/some/absolute".into())
            );
        }

        #[test]
        fn returns_none_at_root() {
            let vp = VirtualPath::Real("/".into());

            assert_eq!(vp.parent(), None);
        }
    }

    mod parent_virtual {
        use super::*;

        #[test]
        fn returns_some_parent() {
            let vp = VirtualPath::Virtual {
                path: "/root/path".into(),
                virtual_prefix: "/root".into(),
                real_prefix: "/some/absolute".into(),
            };

            assert_eq!(
                vp.parent().unwrap(),
                VirtualPath::Virtual {
                    path: "/root".into(),
                    virtual_prefix: "/root".into(),
                    real_prefix: "/some/absolute".into(),
                }
            );
        }

        #[test]
        fn returns_none_at_root() {
            let vp = VirtualPath::Virtual {
                path: "/root".into(),
                virtual_prefix: "/root".into(),
                real_prefix: "/some/absolute".into(),
            };

            assert_eq!(vp.parent(), None);

            let vp = VirtualPath::Virtual {
                path: "/".into(),
                virtual_prefix: "/root".into(),
                real_prefix: "/some/absolute".into(),
            };

            assert_eq!(vp.parent(), None);
        }
    }
}
