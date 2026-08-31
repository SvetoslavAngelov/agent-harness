use serde::Deserialize;
use serde_json::{Value, json};

use super::tool_interface::Tool;
use crate::domain::tool_catalogue::{ToolCallErrorKind, ToolDefinition, ToolError};

#[derive(Deserialize)]
struct ToolArguments {
    city: String,
    country: String,
}

pub struct WeatherCheck;

impl Tool for WeatherCheck {
    fn get_definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "weather_check",
            "Query this function to get the latest weather forecast for any city, or country in Celcius. Use snake_case for the function parameters. For example, london, new_delhi, united_kingdom.",
            json!({
                "type": "object",
                "properties": {
                    "city": {
                        "type": "string"
                    },
                    "country": {
                        "type": "string"
                    }
                },
                "required": ["city", "country"],
                "additionalProperties": false
            }),
        )
    }

    fn execute(&self, arguments: &str) -> Result<Value, ToolError> {
        let arguments: ToolArguments =
            serde_json::from_str(arguments).map_err(|error| ToolError {
                kind: ToolCallErrorKind::InvalidArguments,
                message: error.to_string(),
            })?;

        let temperature =
            weather_check(arguments.city, arguments.country).map_err(|message| ToolError {
                kind: ToolCallErrorKind::ExecutionFailed,
                message,
            })?;

        Ok(json!({
            "temperature_celsius": temperature
        }))
    }
}

fn weather_check(city: String, country: String) -> Result<f64, String> {
    if city == "oxford" && country == "united_kingdom" {
        Ok(34.2)
    } else {
        Err("Can't retrieve temperature.".into())
    }
}
