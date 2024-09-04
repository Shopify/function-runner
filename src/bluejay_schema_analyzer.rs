use crate::scale_limits_analyzer::ScaleLimitsAnalyzer;
use anyhow::{anyhow, Result};
use bluejay_parser::{
    ast::{
        definition::{DefinitionDocument, SchemaDefinition},
        executable::ExecutableDocument,
        Parse,
    },
    Error,
};

pub struct BluejaySchemaAnalyzer;

impl BluejaySchemaAnalyzer {
    pub fn analyze_schema_definition(
        schema_string: &str,
        query: &str,
        input: &serde_json::Value,
    ) -> Result<f64> {
        let document_definition = DefinitionDocument::parse(schema_string)
            .map_err(|errors| anyhow!(Error::format_errors(schema_string, errors)))?;

        let schema_definition = SchemaDefinition::try_from(&document_definition)
            .map_err(|errors| anyhow!(Error::format_errors(schema_string, errors)))?;

        let executable_document = ExecutableDocument::parse(query)
            .map_err(|errors| anyhow!(Error::format_errors(query, errors)))?;

        let cache =
            bluejay_validator::executable::Cache::new(&executable_document, &schema_definition);

        ScaleLimitsAnalyzer::analyze(
            &executable_document,
            &schema_definition,
            None,
            &Default::default(),
            &cache,
            input,
        )
        .map_err(|e| anyhow!(e.message()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_analyze_schema_definition() {
        let schema_string = r#"
            directive @scaleLimits(rate: Float!) on FIELD_DEFINITION
            type Query {
                field: String @scaleLimits(rate: 0.005)
            }
        "#;
        let query = "{ field }";
        let input_json = json!({
            "field": "value"
        });

        let result =
            BluejaySchemaAnalyzer::analyze_schema_definition(schema_string, query, &input_json);
        assert!(
            result.is_ok(),
            "Expected successful analysis but got an error: {:?}",
            result
        );

        let scale_factor = result.unwrap();
        let expected_scale_factor = 1.0;
        assert_eq!(
            scale_factor, expected_scale_factor,
            "The scale factor did not match the expected value"
        );
    }

    #[test]
    fn test_analyze_schema_with_array_length_scaling() {
        let schema_string = r#"
            directive @scaleLimits(rate: Float!) on FIELD_DEFINITION
            type Query {
                cartLines: [String] @scaleLimits(rate: 0.005)
            }
        "#;
        let query = "{ cartLines }";
        let input_json = json!({
            "cartLines": vec!["moeowomeow"; 500]
        });

        let result =
            BluejaySchemaAnalyzer::analyze_schema_definition(schema_string, query, &input_json);
        assert!(
            result.is_ok(),
            "Expected successful analysis but got an error: {:?}",
            result
        );

        let scale_factor = result.unwrap();
        let expected_scale_factor = 2.5; // Adjust this based on how your scale limits are defined
        assert_eq!(
            scale_factor, expected_scale_factor,
            "The scale factor did not match the expected value for array length scaling"
        );
    }

    #[test]
    fn test_analyze_schema_with_array_length_scaling_to_max_scale_factor() {
        let schema_string = r#"
            directive @scaleLimits(rate: Float!) on FIELD_DEFINITION
            type Query {
                cartLines: [String] @scaleLimits(rate: 0.005)
            }
        "#;
        let query = "{ cartLines }";
        let input_json = json!({
            "cartLines": vec!["item"; 1000000] // value that would scale well beyond the max
        });

        let result =
            BluejaySchemaAnalyzer::analyze_schema_definition(schema_string, query, &input_json);
        assert!(
            result.is_ok(),
            "Expected successful analysis but got an error: {:?}",
            result
        );

        let scale_factor = result.unwrap();
        let expected_scale_factor = 10.0;
        assert_eq!(
            scale_factor, expected_scale_factor,
            "The scale factor did not match the expected value for array length scaling"
        );
    }

    #[test]
    fn test_no_double_counting_for_duplicate_fields_with_array() {
        let schema_string = r#"
            directive @scaleLimits(rate: Float!) on FIELD_DEFINITION
            type Query {
                field: [String] @scaleLimits(rate: 0.005)
            }
        "#;
        let query = "{ field field }";
        let input_json = json!({
            "field": vec!["value"; 200]
        });

        let result =
            BluejaySchemaAnalyzer::analyze_schema_definition(schema_string, query, &input_json);
        assert!(
            result.is_ok(),
            "Expected successful analysis but got an error: {:?}",
            result
        );

        let scale_factor = result.unwrap();
        let expected_scale_factor = 1.0;
        assert_eq!(
            scale_factor, expected_scale_factor,
            "The scale factor did not match the expected value, indicating potential double counting"
        );
    }

    #[test]
    fn test_no_double_counting_for_duplicate_fields_with_nested_array() {
        let schema_string = r#"
            directive @scaleLimits(rate: Float!) on FIELD_DEFINITION
            type Query {
                field: [MyObject]
            }

            type MyObject {
                field: [String] @scaleLimits(rate: 0.005)
            }
        "#;
        let query = "{ field { field } }";
        let nested_field = json!({ "field": vec!["value"; 200] });
        let input_json = json!({
            "field": vec![nested_field; 2]
        });

        let result =
            BluejaySchemaAnalyzer::analyze_schema_definition(schema_string, query, &input_json);
        assert!(
            result.is_ok(),
            "Expected successful analysis but got an error: {:?}",
            result
        );

        let scale_factor = result.unwrap();
        let expected_scale_factor = 2.0;
        assert_eq!(
            scale_factor, expected_scale_factor,
            "The scale factor did not match the expected value, indicating potential double counting"
        );
    }
}
