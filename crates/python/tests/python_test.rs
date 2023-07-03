use proto_core::Proto;
use starbase_sandbox::create_empty_sandbox;

mod python {
    use super::*;

    #[test]
    fn it_works() {
        let fixture = create_empty_sandbox();
        let _proto = Proto::from(fixture.path());
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
