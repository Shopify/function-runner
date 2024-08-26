use bluejay_core::definition::ObjectTypeDefinition;
use bluejay_core::definition::SchemaDefinition as CoreSchemaDefinition;
use bluejay_core::AsIter;
use bluejay_core::Directive;
use bluejay_core::Value as CoreValue;
use bluejay_core::{definition::HasDirectives, ValueReference};
use bluejay_parser::error::{Annotation, Error as BluejayError};
use bluejay_parser::{
    ast::{
        definition::FieldDefinition,
        definition::{
            ArgumentsDefinition, DefaultContext, DefinitionDocument, Directives, SchemaDefinition,
        },
        executable::ExecutableDocument,
    },
    Error,
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

pub struct ScaleLimits<'a> {
    input: &'a Value,
    value_stack: Vec<Vec<&'a Value>>,
    rates: HashMap<String, f64>,
}

impl<'a> ScaleLimits<'a> {
    pub fn rates_as_json(&self) -> String {
        serde_json::to_string(&self.rates).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn update_max_rate(&mut self, field_name: &str, rate: f64) {
        let entry = self.rates.entry(field_name.to_string()).or_insert(0.0);
        if rate > *entry {
            *entry = rate;
        }
    }
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
        operation_definition: &'a <ExecutableDocument as bluejay_core::executable::ExecutableDocument>::OperationDefinition,
        schema_definition: &'a SchemaDefinition,
        variable_values: &'a serde_json::Map<String, serde_json::Value>,
        cache: &'a bluejay_validator::executable::Cache<'a, ExecutableDocument, SchemaDefinition>,
        extra_info: &'a Value,
    ) -> Self {
        Self {
            input: extra_info,
            value_stack: vec![vec![extra_info]],
            rates: Default::default(),
        }
    }

    fn visit_field(
        &mut self,
        field: &'_ <ExecutableDocument<'_> as bluejay_core::executable::ExecutableDocument>::Field,
        field_definition: &'_ <SchemaDefinition as CoreSchemaDefinition>::FieldDefinition,
        scoped_type: bluejay_core::definition::TypeDefinitionReference<
            '_,
            <SchemaDefinition<'_> as CoreSchemaDefinition>::TypeDefinition,
        >,
        included: bool,
    ) {
        let rate = Self::rate_for_field_definition(field_definition);
        let values = self.value_stack.last().unwrap();
        let mut nested_values = Vec::new();
        values.iter().for_each(|value| {
            let values_for_field = match value {
                Value::Object(object) => object.get(field.response_key()).into_iter().collect(),
                Value::Null => Vec::new(),
                Value::Array(list) => list.iter().collect(),
                _ => panic!("invalid value"),
            };
            // TODO handle scaling for this value

            nested_values.extend(values_for_field);
        });
    }

    fn leave_field(
        &mut self,
        field: &'a <ExecutableDocument<'a> as bluejay_core::executable::ExecutableDocument>::Field,
        field_definition: &'a <SchemaDefinition<'a> as CoreSchemaDefinition>::FieldDefinition,
        scoped_type: bluejay_core::definition::TypeDefinitionReference<
            'a,
            <SchemaDefinition<'a> as CoreSchemaDefinition>::TypeDefinition,
        >,
        included: bool,
    ) {
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
        0.0
    }
}

impl<'a> ScaleLimits<'a> {
    fn rate_for_field_definition(
        field_definition: &FieldDefinition<DefaultContext>,
    ) -> Option<f64> {
        field_definition.directives().and_then(|directives| {
            directives
                .iter()
                .find(|directive| directive.name() == "scaleLimits")
                .and_then(|directive| directive.arguments())
                .and_then(|arguments| {
                    arguments
                        .iter()
                        .find(|argument| argument.name() == "rate")
                        .and_then(|argument| {
                            if let ValueReference::Float(rate) = argument.value().as_ref() {
                                Some(rate)
                            } else {
                                None
                            }
                        })
                })
        })
    }
}
