use crate::scale_limits_analyzer::ScaleLimitsAnalyzer;
use anyhow::{anyhow, Result};
use bluejay_parser::ast::{
    definition::{DefinitionDocument, SchemaDefinition},
    executable::ExecutableDocument,
    Parse,
};
use serde_json::to_string as to_json_string;

pub struct BluejaySchemaAnalyzer;

impl BluejaySchemaAnalyzer {
    pub fn analyze_schema_definition(
        schema_string: &str,
        query: &str,
        input: &serde_json::Value,
    ) -> Result<f64> {
        let document_definition = DefinitionDocument::parse(schema_string)
            .map_err(|e| anyhow!("Failed to parse schema: {:?}", e))?;

        let schema_definition =
            SchemaDefinition::try_from(&document_definition).map_err(|errors| {
                anyhow!(
                    "Invalid Schema: {:?}",
                    errors
                        .iter()
                        .map(|e| format!("{:?}", e))
                        .collect::<Vec<_>>()
                )
            })?;

        let executable_document = ExecutableDocument::parse(query)
            .map_err(|e| anyhow!("Error parsing query: {:?}", e))?;

        let cache =
            bluejay_validator::executable::Cache::new(&executable_document, &schema_definition);

        let input_str = to_json_string(input).unwrap_or_else(|_| "<invalid JSON>".to_string());

        ScaleLimitsAnalyzer::analyze(
            &executable_document,
            &schema_definition,
            None,
            &Default::default(),
            &cache,
            input,
        )
        .map_err(|e| anyhow!("Error analyzing scale limits: Input: {} {:?}", input_str, e))
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
}
