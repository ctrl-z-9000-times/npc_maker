use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Error type for manipulating JSON files.
#[derive(thiserror::Error, Debug)]
pub enum JsonIoError {
    #[error("message")]
    Json(#[from] serde_json::Error),

    #[error("message")]
    Io(#[from] std::io::Error),
}

/// Custom default value for serde.  
/// Usage: `#[serde(default="default_one")]`  
pub fn default_one() -> f64 {
    1.0
}

/// Converts between common symbolic representations of magic numbers and their
/// actual f64 values.
pub mod f64_symbols {
    use super::*;

    pub fn serialize<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if *value == f64::INFINITY {
            serializer.serialize_str("inf")
        } else if *value == f64::NEG_INFINITY {
            serializer.serialize_str("-inf")
        } else if *value == std::f64::consts::PI {
            serializer.serialize_str("pi")
        } else if *value == 2.0 * std::f64::consts::PI {
            serializer.serialize_str("tau")
        } else {
            serializer.serialize_f64(*value)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum Number {
            #[serde(rename = "-inf")]
            NegInf,

            #[serde(rename = "inf")]
            PosInf,

            #[serde(rename = "pi")]
            Pi,

            #[serde(rename = "tau")]
            Tau,

            #[serde(untagged)]
            Finite(f64),
        }

        let num = Number::deserialize(deserializer)?;
        Ok(match num {
            Number::PosInf => f64::INFINITY,
            Number::NegInf => f64::NEG_INFINITY,
            Number::Pi => std::f64::consts::PI,
            Number::Tau => 2.0 * std::f64::consts::PI,
            Number::Finite(value) => value,
        })
    }
}

/// Usage: `#[serde(default, with = "OptionF64Symbols")]`
#[derive(Serialize, Deserialize)]
#[serde(untagged, remote = "Option<f64>")]
pub enum OptionF64Symbols {
    None,
    Some(#[serde(with = "f64_symbols")] f64),
}

/// Enforces the constraint that the given value is in the range [0, 1].
///
/// Usage: `#[serde(deserialize_with = "deserialize_fraction")]`
pub fn deserialize_fraction<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = f64::deserialize(deserializer)?;
    if value < 0.0 {
        Err(serde::de::Error::custom("value not in range [0, 1]"))
    } else if value > 1.0 {
        Err(serde::de::Error::custom("value not in range [0, 1]"))
    } else {
        Ok(value)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged, remote = "Option<f64>")]
pub enum OptionFraction {
    None,
    Some(#[serde(deserialize_with = "deserialize_fraction")] f64),
}

/// Enforces the constraint that the given value is greater than zero.
///
/// Usage: `#[serde(deserialize_with = "deserialize_positive")]`
pub fn deserialize_positive<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = f64::deserialize(deserializer)?;
    if value < 0.0 {
        Err(serde::de::Error::custom("value less than zero"))
    } else {
        Ok(value)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged, remote = "Option<f64>")]
pub enum OptionPositive {
    None,
    Some(#[serde(deserialize_with = "deserialize_positive")] f64),
}

static ALL_STRINGS: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);

/// Deserializes a string and leak it so that it lives forever.  
/// This also dedups / reuses the strings to avoid wasting memory.
///
/// Usage:  #[serde(deserialize_with = "static_str")]
pub fn static_str<'de, D>(deserializer: D) -> Result<&'static str, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct BorrowedStr<'a>(&'a str);
    let value = BorrowedStr::deserialize(deserializer)?;
    Ok(get_static_str(value.0))
}

pub fn get_static_str(value: &str) -> &'static str {
    let mut all_strings = ALL_STRINGS.lock().unwrap();
    // Lazy init the global table of all strings.
    if all_strings.is_none() {
        *all_strings = Some(HashSet::with_capacity(1));
    }
    // Get this string from the global table.
    let all_strings = all_strings.as_mut().unwrap();
    match all_strings.get(value) {
        Some(static_str) => static_str,
        None => {
            // Use a "Box" instead of a "String" to avoid allocating extra capacity.
            let static_str = Box::leak(value.into());
            all_strings.insert(static_str);
            static_str
        }
    }
}

/// Enforces the constraint that the given string is not empty.
///
/// Usage: `#[serde(deserialize_with = "required_string")]`
pub fn required_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        Err(serde::de::Error::custom("missing required string"))
    } else {
        Ok(value)
    }
}

/// Allow a list of strings in place of a single string, to reduce the line
/// length of long descriptions. The elements are simply concatenated together.
///
/// Usage: `#[serde(deserialize_with = "multiline_string")]`
pub fn multiline_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrList {
        String(String),
        List(Vec<String>),
    }
    let value = StringOrList::deserialize(deserializer)?;
    match value {
        StringOrList::String(value) => Ok(value),
        StringOrList::List(value) => Ok(value.join("")),
    }
}

/// Usage: `#[serde(deserialize_with = "multiline_arc_str")]`
pub fn multiline_arc_str<'de, D>(deserializer: D) -> Result<Arc<str>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = multiline_string(deserializer)?;
    Ok(value.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_f64() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Container {
            #[serde(default, with = "OptionF64Symbols")]
            data: Option<f64>,
        }
        for data in [
            None,
            Some(0.0),
            Some(42.0),
            Some(-7.0),
            Some(f64::INFINITY),
            Some(f64::NEG_INFINITY),
            Some(std::f64::consts::PI),
            Some(2.0 * std::f64::consts::PI),
        ] {
            let value = dbg!(Container { data });
            let json_str = dbg!(serde_json::to_string(&value)).unwrap();
            let roundtrip: Container = dbg!(serde_json::from_str(&json_str)).unwrap();
            assert_eq!(value, roundtrip);
        }
    }

    #[test]
    fn fraction() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        #[serde(transparent)]
        struct Container {
            #[serde(deserialize_with = "deserialize_fraction")]
            value: f64,
        }

        for valid_str in [
            "0",
            "0.0",
            "-0.0",
            "5e-324", // 0.0_f64.next_up()
            "0.5",
            "  \n\r \n \r\n \t 0.5 \t \n  ", // Leading & trailing whitespace
            "1.0",
            "0.9999999999999999", // 1.0_f64.next_down()
            "0.99999999999999999999999999999999999",
        ] {
            let _valid_value: Container = dbg!(serde_json::from_str(&dbg!(valid_str))).unwrap();
        }
        for invalid_str in [
            "-0.1",
            "1.1",
            "-1.7976931348623157e308", // f64::MIN
            "1.7976931348623157e308",  // f64::MAX
            "-5e-324",                 // 0.0_f64::next_down()
            "1.0000000000000002",      // 0.0_f64::next_up()
        ] {
            let invalid_error: Result<Container, _> =
                dbg!(serde_json::from_str(&dbg!(invalid_str)));
            if let Err(msg) = &invalid_error {
                eprintln!("{msg}"); // Check error message formatting.
                eprintln!("{msg:?}"); // Check error message formatting.
            }
            assert!(invalid_error.is_err());
        }
    }

    #[test]
    fn static_str() {
        #[derive(Serialize, Deserialize, Debug)]
        struct Foo {
            #[serde(deserialize_with = "super::static_str")]
            a: &'static str,
            #[serde(deserialize_with = "super::static_str")]
            b: &'static str,
            #[serde(deserialize_with = "super::static_str")]
            c: &'static str,
        }
        let foo_str = r#"{"a": "foobar", "b": "foobar", "c": "test123"}"#;
        let foo_val: Foo = serde_json::from_str(foo_str).unwrap();

        assert_eq!(foo_val.a, "foobar");
        assert_eq!(foo_val.a.as_ptr(), foo_val.b.as_ptr());
        assert_eq!(foo_val.c, "test123");
        assert_eq!(
            foo_val.a.as_ptr(),
            get_static_str(&("foobar".to_string())).as_ptr()
        );
    }

    #[test]
    fn multiline_str() {
        #[derive(Serialize, Deserialize, Debug)]
        struct Foo {
            #[serde(deserialize_with = "multiline_string")]
            x: String,
        }
        let singl_line = r#"{"x": "foobar"}"#;
        let multi_line = r#"{"x": ["one fish",
                                    " two fish"]}"#;
        let singl_foo: Foo = serde_json::from_str(singl_line).unwrap();
        let multi_foo: Foo = serde_json::from_str(multi_line).unwrap();
        assert_eq!(singl_foo.x, "foobar");
        assert_eq!(multi_foo.x, "one fish two fish");
    }
}
