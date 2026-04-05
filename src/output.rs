use std::fmt;
use std::str::FromStr;
use clap::ValueEnum;

/// Enumerator used for --output option
#[derive(Clone, Debug, Copy, PartialEq, Default, ValueEnum)]
pub enum Output {
    #[default]
    Text,
    Json,
    /// Json Max: The maximum amount of output in Json format
    JsonMax,
    /// Json Spec:Matrix Specifications
    JsonSpec,
}

macro_rules! is_variant {
    ($name:ident, $variant:ident) => {
        pub fn $name(&self) -> bool {
            matches!(self, Self::$variant)
        }
    };
}

impl Output {
    is_variant!(is_text, Text);
    is_variant!(is_json, Json);
    is_variant!(is_json_max, JsonMax);
    is_variant!(is_json_spec, JsonSpec);
}

impl FromStr for Output {
    type Err = &'static str;
    fn from_str(src: &str) -> Result<Output, &'static str> {
        match src.to_lowercase().as_str() {
            "text" => Ok(Output::Text),
            "json" => Ok(Output::Json),
            s if ["jsonmax", "json-max", "json_max"].contains(&s) => Ok(Output::JsonMax),
            s if ["jsonspec", "json_spec", "json-spec"].contains(&s) => Ok(Output::JsonSpec),
            _ => Err("Invalid output format"),
        }
    }
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Output::Text => write!(f, "text"),
            Output::Json => write!(f, "json"),
            Output::JsonMax => write!(f, "json-max"),
            Output::JsonSpec => write!(f, "json-spec"),
        }
    }
}
