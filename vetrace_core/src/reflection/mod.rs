mod dynamic_value;
mod error;
mod field_path;
mod humanize;
mod schema;

pub use dynamic_value::{merge_dynamic_patch, DynamicValue};
pub use error::ReflectionError;
pub use field_path::{FieldPath, FieldSegment};
pub use humanize::humanize_identifier;
pub use schema::{
    ComponentSchema, FieldKind, FieldSchema, NumericRange, VetraceComponent, VetraceEnum,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_paths_read_and_write_nested_values() {
        let mut value = DynamicValue::from_json(serde_json::json!({
            "items": [{"amount": 2}],
            "position": [1.0, 2.0, 3.0]
        }));
        let amount = FieldPath::parse("items[0].amount").unwrap();
        assert_eq!(value.get(&amount).unwrap(), &DynamicValue::I64(2));
        value.set(&amount, DynamicValue::I64(7)).unwrap();
        assert_eq!(value.get(&amount).unwrap(), &DynamicValue::I64(7));
    }

    #[test]
    fn dynamic_values_round_trip_json_numbers() {
        let source = serde_json::json!({"signed": -3, "unsigned": 4, "float": 1.5});
        let dynamic = DynamicValue::from_json(source.clone());
        assert_eq!(dynamic.into_json(), source);
    }
}
