use crate::ecs::Component;
use crate::inspector::{Inspectable, export::{ExportedField, ExportKind}};
use std::any::TypeId;
use ahash::HashMap;

#[derive(Clone, Copy)]
pub enum FieldType {
    F32,
    I32,
    Bool,
}

#[derive(Clone)]
pub struct GeneratedSpec {
    pub fields: Vec<(&'static str, FieldType)>,
}

impl GeneratedSpec {
    pub fn instance(&self) -> GeneratedComponent {
        let values = self
            .fields
            .iter()
            .map(|(_, t)| match t {
                FieldType::F32 => FieldValue::F32(0.0),
                FieldType::I32 => FieldValue::I32(0),
                FieldType::Bool => FieldValue::Bool(false),
            })
            .collect();
        GeneratedComponent { spec: self.clone(), values }
    }
}

pub enum FieldValue {
    F32(f32),
    I32(i32),
    Bool(bool),
}

pub struct GeneratedComponent {
    pub spec: GeneratedSpec,
    pub values: Vec<FieldValue>,
}

impl Component for GeneratedComponent {}

impl Inspectable for GeneratedComponent {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        self.spec
            .fields
            .iter()
            .enumerate()
            .map(|(i, (name, ty))| match (&ty, &mut self.values[i]) {
                (FieldType::F32, FieldValue::F32(v)) => ExportedField {
                    name,
                    kind: ExportKind::Slider { min: 0.0, max: 100.0 },
                    value: v as *mut _ as *mut dyn std::any::Any,
                    type_id: TypeId::of::<f32>(),
                },
                (FieldType::I32, FieldValue::I32(v)) => ExportedField {
                    name,
                    kind: ExportKind::Slider { min: 0.0, max: 100.0 },
                    value: v as *mut _ as *mut dyn std::any::Any,
                    type_id: TypeId::of::<i32>(),
                },
                (FieldType::Bool, FieldValue::Bool(v)) => ExportedField {
                    name,
                    kind: ExportKind::Checkbox,
                    value: v as *mut _ as *mut dyn std::any::Any,
                    type_id: TypeId::of::<bool>(),
                },
                _ => unreachable!(),
            })
            .collect()
    }
}

#[derive(Default)]
pub struct GeneratedStorage {
    pub components: HashMap<String, GeneratedComponent>,
}

impl Component for GeneratedStorage {}
