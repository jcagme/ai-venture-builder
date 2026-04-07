use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

fn string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Helper;

    impl<'de> Visitor<'de> for Helper {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or a sequence of strings")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![v.to_string()])
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![v])
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values = Vec::new();
            while let Some(elem) = seq.next_element::<String>()? {
                values.push(elem);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_any(Helper)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Idea {
    pub title: String,
    pub problem: String,
    pub solution: String,
    pub target_users: String,
    pub monetization: String,
    #[serde(deserialize_with = "string_or_vec")]
    pub mvp_scope: Vec<String>,
}
