use proto_pdk_api::Checksum;
use std::str::FromStr;

#[test]
#[should_panic(expected = "MissingAlgorithm")]
fn errors_missing_algo() {
    Checksum::from_str("hash").unwrap();
}

#[test]
#[should_panic(expected = "UnsupportedAlgorithm")]
fn errors_unknown_algo() {
    Checksum::from_str("algo:hash").unwrap();
}

#[test]
fn parses_minisign() {
    assert_eq!(
        Checksum::from_str("minisign:RWSGOq2NVecA2UPNdBUZykf1CCb147pkmdtYxgb3Ti+JO/wCYvhbAb/U")
            .unwrap(),
        Checksum::minisign("RWSGOq2NVecA2UPNdBUZykf1CCb147pkmdtYxgb3Ti+JO/wCYvhbAb/U".into())
    );
}

#[test]
fn parses_sha256() {
    assert_eq!(
        Checksum::from_str("1a3f59c07a93ae86c43651a497823792ec9cfd754ece50b51354de8e854fcf1e")
            .unwrap(),
        Checksum::sha256("1a3f59c07a93ae86c43651a497823792ec9cfd754ece50b51354de8e854fcf1e".into())
    );

    assert_eq!(
        Checksum::from_str(
            "sha256:1a3f59c07a93ae86c43651a497823792ec9cfd754ece50b51354de8e854fcf1e"
        )
        .unwrap(),
        Checksum::sha256("1a3f59c07a93ae86c43651a497823792ec9cfd754ece50b51354de8e854fcf1e".into())
    );
}

#[test]
fn parses_sha512() {
    assert_eq!(
        Checksum::from_str("85d7de8e96b19a450d4cea0adc09f5bd6ab8a3bff4cfc164a1533f2099b136856ecbba8d1caf33fb738b78513ea34f751a5e30ca962f0072c70e93cded503880")
            .unwrap(),
        Checksum::sha512("85d7de8e96b19a450d4cea0adc09f5bd6ab8a3bff4cfc164a1533f2099b136856ecbba8d1caf33fb738b78513ea34f751a5e30ca962f0072c70e93cded503880".into())
    );

    assert_eq!(
        Checksum::from_str(
            "sha512:85d7de8e96b19a450d4cea0adc09f5bd6ab8a3bff4cfc164a1533f2099b136856ecbba8d1caf33fb738b78513ea34f751a5e30ca962f0072c70e93cded503880"
        )
        .unwrap(),
        Checksum::sha512("85d7de8e96b19a450d4cea0adc09f5bd6ab8a3bff4cfc164a1533f2099b136856ecbba8d1caf33fb738b78513ea34f751a5e30ca962f0072c70e93cded503880".into())
    );
}
