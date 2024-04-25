pub mod toml_to_string {

    use toml::Value;

    pub fn string_from_toml_value(value: &Value) -> String {
        match value {
            Value::Integer(integer_value) => integer_value.to_string(),
            Value::Float(float_value) => float_value.to_string(),
            Value::Boolean(boolean_value) => boolean_value.to_string(),
            Value::Datetime(datetime_value) => datetime_value.to_string(),
            other => other.as_str().expect(crate::INVALID_TOML_ERROR).to_string(),
        }
    }
}
