use bluejay_core::{
    definition::{prelude::*, SchemaDefinition as CoreSchemaDefinition},
    AsIter, Directive, Value as CoreValue, ValueReference,
};
use bluejay_parser::ast::{
    definition::FieldDefinition,
    definition::{DefaultContext, SchemaDefinition},
    executable::ExecutableDocument,
};
use serde_json::Value;
use std::collections::HashMap;

pub type ScaleLimitsAnalyzer<'a> = bluejay_validator::executable::operation::Orchestrator<
    'a,
    ExecutableDocument<'a>,
    SchemaDefinition<'a>,
    serde_json::Map<String, serde_json::Value>,
    ScaleLimits<'a>,
>;

#[derive(Hash, PartialEq, Eq, Debug)]
struct PathWithIndex<'a> {
    path: Vec<&'a str>,
    index: usize,
}

pub struct ScaleLimits<'a> {
    value_stack: Vec<Vec<&'a Value>>,
    path_stack: Vec<&'a str>,
    rates: HashMap<PathWithIndex<'a>, f64>,
}

impl<'a>
    bluejay_validator::executable::operation::Visitor<
        'a,
        ExecutableDocument<'a>,
        SchemaDefinition<'a>,
        serde_json::Map<String, serde_json::Value>,
    > for ScaleLimits<'a>
{
    type ExtraInfo = &'a Value;

    fn new(
        _operation_definition: &'a <ExecutableDocument as bluejay_core::executable::ExecutableDocument>::OperationDefinition,
        _schema_definition: &'a SchemaDefinition<'a>,
        _variable_values: &'a serde_json::Map<String, serde_json::Value>,
        _cache: &'a bluejay_validator::executable::Cache<'a, ExecutableDocument, SchemaDefinition>,
        extra_info: &'a Value,
    ) -> Self {
        Self {
            value_stack: vec![vec![extra_info]],
            path_stack: Vec::new(),
            rates: Default::default(),
        }
    }

    fn visit_field(
        &mut self,
        field: &'a <ExecutableDocument<'a> as bluejay_core::executable::ExecutableDocument>::Field,
        field_definition: &'_ <SchemaDefinition as CoreSchemaDefinition>::FieldDefinition,
        _scoped_type: bluejay_core::definition::TypeDefinitionReference<
            '_,
            <SchemaDefinition<'_> as CoreSchemaDefinition>::TypeDefinition,
        >,
        _included: bool,
    ) {
        self.path_stack.push(field.response_key());
        let rate = Self::rate_for_field_definition(field_definition);
        let values = self.value_stack.last().unwrap();
        let mut nested_values = Vec::new();

        values.iter().enumerate().for_each(|(index, value)| {
            let value_for_field = match value {
                Value::Object(object) => object.get(field.response_key()),
                Value::Null => None,
                _ => None,
            };
            if let Some(rate) = rate {
                let length = match value_for_field {
                    Some(Value::String(s)) => s.len(),
                    Some(Value::Array(arr)) => arr.len(),
                    _ => 1,
                };
                let increment = length as f64 * rate;

                let path_with_index = PathWithIndex {
                    path: self.path_stack.clone(),
                    index,
                };

                let entry = self.rates.entry(path_with_index).or_default();

                *entry = entry.max(increment);
            }

            match value_for_field {
                Some(Value::Array(values)) => nested_values.extend(values),
                Some(value) => nested_values.push(value),
                None => {}
            }
        });

        self.value_stack.push(nested_values);
    }

    fn leave_field(
        &mut self,
        _field: &'a <ExecutableDocument<'a> as bluejay_core::executable::ExecutableDocument>::Field,
        _field_definition: &'a <SchemaDefinition<'a> as CoreSchemaDefinition>::FieldDefinition,
        _scoped_type: bluejay_core::definition::TypeDefinitionReference<
            'a,
            <SchemaDefinition<'a> as CoreSchemaDefinition>::TypeDefinition,
        >,
        _included: bool,
    ) {
        self.path_stack.pop().unwrap();
        self.value_stack.pop().unwrap();
    }
}

impl<'a>
    bluejay_validator::executable::operation::Analyzer<
        'a,
        ExecutableDocument<'a>,
        SchemaDefinition<'a>,
        serde_json::Map<String, serde_json::Value>,
    > for ScaleLimits<'a>
{
    type Output = f64;

    fn into_output(self) -> Self::Output {
        let normalized_rates = self.rates.into_iter().fold(
            HashMap::new(),
            |mut normalized_rates, (PathWithIndex { path, .. }, rate)| {
                *normalized_rates.entry(path).or_default() += rate;
                normalized_rates
            },
        );

        normalized_rates
            .into_values()
            .fold(Self::MIN_SCALE_FACTOR, f64::max)
            .clamp(Self::MIN_SCALE_FACTOR, Self::MAX_SCALE_FACTOR)
    }
}

impl<'a> ScaleLimits<'a> {
    const MIN_SCALE_FACTOR: f64 = 1.0;
    const MAX_SCALE_FACTOR: f64 = 10.0;

    fn rate_for_field_definition(
        field_definition: &FieldDefinition<DefaultContext>,
    ) -> Option<f64> {
        field_definition
            .directives()
            .iter()
            .flat_map(|directives| directives.iter())
            .find(|directive| directive.name() == "scaleLimits")
            .and_then(|directive| directive.arguments())
            .and_then(|arguments| arguments.iter().find(|argument| argument.name() == "rate"))
            .and_then(|argument| {
                if let ValueReference::Float(rate) = argument.value().as_ref() {
                    Some(rate)
                } else {
                    None
                }
            })
    }
}
