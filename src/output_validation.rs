use anyhow::{anyhow, Result as AnyhowResult};
use bluejay_core::{
    definition::{
        ArgumentsDefinition, FieldDefinition, FieldsDefinition, InputValueDefinition,
        ObjectTypeDefinition, ScalarTypeDefinition, SchemaDefinition,
    },
    AbstractValue, Value,
};
use bluejay_parser::ast::definition::{
    Context, CustomScalarTypeDefinition, DefinitionDocument,
    SchemaDefinition as ParserSchemaDefinition,
};
use bluejay_validator::value::input_coercion::{CoerceInput, Error as GraphqlError, PathMember};
use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fs, path::PathBuf};

#[derive(Debug)]
struct CustomContext;

impl Context for CustomContext {
    fn coerce_custom_scalar_input<const CONST: bool>(
        cstd: &CustomScalarTypeDefinition<Self>,
        value: &impl AbstractValue<CONST>,
    ) -> Result<(), Cow<'static, str>> {
        let value = value.as_ref();
        match cstd.name() {
            "Decimal" => {
                if let Value::String(s) = value {
                    s.parse::<f64>()
                        .map_err(|_| Cow::Owned(format!("Unable to parse `{s}` to Decimal")))
                        .and_then(|f| {
                            if f.is_finite() {
                                Ok(())
                            } else {
                                Err(Cow::Borrowed("Decimal values must be finite"))
                            }
                        })
                } else {
                    Err(Cow::Owned(format!("Cannot coerce {value} to Decimal")))
                }
            }
            "ID" => {
                if let Value::String(s) = value {
                    validate_shopify_gid(s)
                } else {
                    Err(Cow::Owned(format!("Cannot coerce {value} to ID")))
                }
            }
            _ => Ok(()),
        }
    }
}

fn validate_shopify_gid(gid: &str) -> Result<(), Cow<'static, str>> {
    let re = Regex::new(r"^gid://shopify/([a-zA-Z0-9_]+)/([a-zA-Z0-9]+)$").unwrap();

    if re.is_match(gid) {
        Ok(())
    } else {
        Err(Cow::Borrowed("Invalid GID format"))
    }
}

pub fn validate_output(
    value: &serde_json::Value,
    schema_path: &PathBuf,
) -> AnyhowResult<Result<(), Vec<OutputValidationError>>> {
    let schema_string = fs::read_to_string(schema_path)
        .map_err(|e| anyhow!("Couldn't load schema {:?}: {}", schema_path, e))?;

    let definition_document: DefinitionDocument<CustomContext> =
        DefinitionDocument::parse(&schema_string)
            .map_err(|_| anyhow!("Error parsing schema document"))?;

    let schema_definition: ParserSchemaDefinition<CustomContext> =
        ParserSchemaDefinition::try_from(&definition_document).map_err(|errs| {
            dbg!(errs);
            anyhow!("Schema document is invalid")
        })?;

    let function_result_type_reference = schema_definition
        .mutation()
        .ok_or_else(|| anyhow!("Schema document does not define a mutation root"))?
        .fields_definition()
        .get("handleResult")
        .ok_or_else(|| {
            anyhow!("Schema document mutation root does not define a field named `handleResult`")
        })?
        .arguments_definition()
        .and_then(|arguments_definition| arguments_definition.get("result"))
        .ok_or_else(|| anyhow!("Schema document mutation root field `handleResult` does not define an argument named `result`"))?
        .r#type();

    let result = function_result_type_reference
        .coerce_value(value, &[])
        .map_err(|errors| {
            errors
                .into_iter()
                .map(OutputValidationError::from)
                .collect()
        });

    Ok(result)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutputValidationError {
    message: Cow<'static, str>,
    path: Vec<String>,
}

impl OutputValidationError {
    pub fn new(message: impl Into<Cow<'static, str>>, path: Vec<PathMember>) -> Self {
        Self {
            message: message.into(),
            path: path
                .into_iter()
                .map(|member| match member {
                    PathMember::Index(i) => i.to_string(),
                    PathMember::Key(k) => k.to_string(),
                })
                .collect(),
        }
    }
}

impl<'a> From<GraphqlError<'a, true, serde_json::Value>> for OutputValidationError {
    fn from(value: GraphqlError<'a, true, serde_json::Value>) -> Self {
        match value {
            GraphqlError::CustomScalarInvalidValue { message, path, .. } => {
                Self::new(message, path)
            }
            GraphqlError::NoEnumMemberWithName {
                name,
                enum_type_name,
                path,
                ..
            } => Self::new(
                format!("No enum member `{name}` on type {enum_type_name}"),
                path,
            ),
            GraphqlError::NoImplicitConversion {
                value,
                input_type_name,
                path,
            } => Self::new(
                format!(
                    "No implicit conversion of {} to {input_type_name}",
                    AbstractValue::<true>::as_ref(value)
                ),
                path,
            ),
            GraphqlError::NoInputFieldWithName {
                field,
                input_object_type_name,
                path,
            } => Self::new(
                format!("No field with name {field} on input type {input_object_type_name}",),
                path,
            ),
            GraphqlError::NoValueForRequiredFields {
                field_names,
                input_object_type_name,
                path,
                ..
            } => {
                let joined_field_names = field_names.into_iter().join(", ");
                Self::new(
                    format!("No value for required fields on input type {input_object_type_name}: {joined_field_names}"),
                    path,
                )
            }
            GraphqlError::NonUniqueFieldNames { .. } => {
                unreachable!("Should not be possible for serde_json::Value")
            }
            GraphqlError::NullValueForRequiredType {
                input_type_name,
                path,
                ..
            } => Self::new(
                format!("Got null when non-null value of type {input_type_name} was expected"),
                path,
            ),
            GraphqlError::OneOfInputNotSingleNonNullValue {
                input_object_type_name,
                non_null_entries,
                path,
                ..
            } => {
                if non_null_entries.is_empty() {
                    Self::new(
                        format!("No entries with non-null values for oneOf input object {input_object_type_name}"),
                        path,
                    )
                } else {
                    let entry_names = non_null_entries.into_iter().map(|(key, _)| key).join(", ");
                    Self::new(
                        format!("Multiple entries with non-null values for oneOf input object {input_object_type_name}: {entry_names}"),
                        path,
                    )
                }
            }
            GraphqlError::OneOfInputNullValues {
                input_object_type_name,
                null_entries,
                path,
                ..
            } => {
                let entry_names = null_entries.into_iter().map(|(key, _)| key).join(", ");
                Self::new(
                    format!("Multiple entries with null values for oneOf input object {input_object_type_name}: {entry_names}"),
                    path
                )
            }
        }
    }
}
