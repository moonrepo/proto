#[macro_export]
macro_rules! impl_id_parse {
    ($name:ident, $error:ident) => {
        #[automatically_derived]
        impl std::str::FromStr for $name {
            type Err = $error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                $name::new(s)
            }
        }

        #[automatically_derived]
        impl TryFrom<String> for $name {
            type Error = $error;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                $name::from_str(&value)
            }
        }

        #[automatically_derived]
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                $name::new(String::deserialize(deserializer)?)
                    .map_err(|error| serde::de::Error::custom(error.to_string()))
            }
        }
    };
}
