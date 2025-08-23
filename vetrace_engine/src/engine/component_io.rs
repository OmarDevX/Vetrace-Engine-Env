use serde_json::{Map, Value};
use crate::inspector::Inspectable;

/// Apply JSON component data to a component implementing [`Inspectable`].
pub fn apply_component_data(comp: &mut dyn Inspectable, data: &Value) {
    if let Value::Object(map) = data {
        for field in comp.exported_fields_mut() {
            if let Some(val) = map.get(field.name) {
                unsafe {
                    if field.type_id == std::any::TypeId::of::<f32>() {
                        if let Some(v) = val.as_f64() {
                            *(field.value as *mut f32) = v as f32;
                        }
                    } else if field.type_id == std::any::TypeId::of::<f64>() {
                        if let Some(v) = val.as_f64() {
                            *(field.value as *mut f64) = v;
                        }
                    } else if field.type_id == std::any::TypeId::of::<i32>() {
                        if let Some(v) = val.as_i64() {
                            *(field.value as *mut i32) = v as i32;
                        }
                    } else if field.type_id == std::any::TypeId::of::<u32>() {
                        if let Some(v) = val.as_u64() {
                            *(field.value as *mut u32) = v as u32;
                        }
                    } else if field.type_id == std::any::TypeId::of::<bool>() {
                        if let Some(v) = val.as_bool() {
                            *(field.value as *mut bool) = v;
                        }
                    } else if field.type_id == std::any::TypeId::of::<String>() {
                        if let Some(v) = val.as_str() {
                            *(field.value as *mut String) = v.to_string();
                        }
                    }
                }
            }
        }
    }
}

/// Export component data as a JSON [`Value`].
pub fn export_component_data(comp: &mut dyn Inspectable) -> Value {
    let mut map = Map::new();
    for field in comp.exported_fields_mut() {
        unsafe {
            let val = if field.type_id == std::any::TypeId::of::<f32>() {
                Value::from(*(field.value as *mut f32))
            } else if field.type_id == std::any::TypeId::of::<f64>() {
                Value::from(*(field.value as *mut f64))
            } else if field.type_id == std::any::TypeId::of::<i32>() {
                Value::from(*(field.value as *mut i32))
            } else if field.type_id == std::any::TypeId::of::<u32>() {
                Value::from(*(field.value as *mut u32))
            } else if field.type_id == std::any::TypeId::of::<bool>() {
                Value::from(*(field.value as *mut bool))
            } else if field.type_id == std::any::TypeId::of::<String>() {
                Value::from((*(field.value as *mut String)).clone())
            } else {
                continue;
            };
            map.insert(field.name.to_string(), val);
        }
    }
    Value::Object(map)
}
