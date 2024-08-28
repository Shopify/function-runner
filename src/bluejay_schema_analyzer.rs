use crate::scale_limits_analyzer::ScaleLimitsAnalyzer;
use bluejay_parser::error::Error as BluejayError;
use bluejay_parser::{
    ast::{
        definition::{DefaultContext, DefinitionDocument, SchemaDefinition},
        executable::ExecutableDocument,
        Parse,
    },
    Error,
};

pub struct BluejaySchemaAnalyzer;

impl BluejaySchemaAnalyzer {
    pub fn create_definition_document(
        schema_string: &str,
    ) -> Result<DefinitionDocument, Vec<Error>> {
        DefinitionDocument::parse(schema_string)
    }

    pub fn create_schema_definition<'a>(
        definition_document: &'a DefinitionDocument,
    ) -> Result<SchemaDefinition<'a, DefaultContext>, Vec<BluejayError>> {
        SchemaDefinition::try_from(definition_document).map_err(|errors| {
            errors
                .into_iter()
                .map(|e| BluejayError::new(format!("Invalid Schema: {:?}", e), None, Vec::new()))
                .collect()
        })
    }

    pub fn analyze_schema_definition(
        schema_definition: SchemaDefinition,
        query: &str,
        input: &serde_json::Value,
    ) -> Result<f64, Error> {
        let executable_document = ExecutableDocument::parse(query);

        match executable_document {
            Ok(ed) => {
                let cache = bluejay_validator::executable::Cache::new(&ed, &schema_definition);

                ScaleLimitsAnalyzer::analyze(
                    &ed,
                    &schema_definition,
                    None,
                    &Default::default(),
                    &cache,
                    input,
                )
                .map_err(|e| {
                    Error::new(
                        format!("Error analyzing scale limits: {:?}", e),
                        None,
                        Vec::new(),
                    )
                })
            }
            Err(_e) => Err(Error::new(
                "Error creating the executable document",
                None,
                Vec::new(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_definition_document_valid() {
        let schema = r#"
        type Query {
            field: String @scaleLimits(rate: 1.5)
        }
        "#;
        let result = BluejaySchemaAnalyzer::create_definition_document(schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_definition_document_invalid() {
        let schema = "type Query { field: String";
        let result = BluejaySchemaAnalyzer::create_definition_document(schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_schema_definition_valid() {
        let schema_string = r#"
            directive @scaleLimits(rate: Float!) on FIELD_DEFINITION

            type Query {
                field: String @scaleLimits(rate: 0.005)
            }
        "#;
        let definition_document = BluejaySchemaAnalyzer::create_definition_document(schema_string)
            .expect("Schema had parse errors");
        let result = BluejaySchemaAnalyzer::create_schema_definition(&definition_document);
        eprintln!("{:?}", result);

        assert!(result.is_ok());
    }

    #[test]
    fn test_create_schema_definition_invalid() {
        let schema_string = r#"
            type Query {
                field: String @scaleLimits(rate: "invalid_rate")
            }
        "#;
        let definition_document =
            BluejaySchemaAnalyzer::create_definition_document(schema_string).unwrap();
        let result = BluejaySchemaAnalyzer::create_schema_definition(&definition_document);
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_schema_definition() {
        let schema_string = r#"
            directive @scaleLimits(rate: Float!) on FIELD_DEFINITION

            type Query {
                field: String @scaleLimits(rate: 0.005)
            }
        "#;
        let query = "{ field }";
        let input_json = serde_json::json!({
            "field": "value"
        });

        let definition_document =
            BluejaySchemaAnalyzer::create_definition_document(schema_string).unwrap();
        let schema_definition =
            BluejaySchemaAnalyzer::create_schema_definition(&definition_document).unwrap();
        let result =
            BluejaySchemaAnalyzer::analyze_schema_definition(schema_definition, query, &input_json);

        eprintln!("result => {:?}", result);

        assert!(
            result.is_ok(),
            "Expected successful analysis but got an error: {:?}",
            result
        );

        let scale_factor = result.unwrap();
        let expected_scale_factor = 1.0; // This should be the expected result based on your application logic
        assert_eq!(
            scale_factor, expected_scale_factor,
            "The scale factor did not match the expected value"
        );
    }

    #[test]
    fn test_analyze_schema_with_large_input() {
        let schema_string = r#"
            directive @scaleLimits(rate: Float!) on FIELD_DEFINITION

            type Query {
                field: [String] @scaleLimits(rate: 0.005)
            }
        "#;
        let query = "{ field }";
        let input_json = serde_json::json!({
          "field": vec!["value"; 10000]
        });

        let definition_document =
            BluejaySchemaAnalyzer::create_definition_document(schema_string).unwrap();
        let schema_definition =
            BluejaySchemaAnalyzer::create_schema_definition(&definition_document).unwrap();
        let result =
            BluejaySchemaAnalyzer::analyze_schema_definition(schema_definition, query, &input_json);

        assert!(
            result.is_ok(),
            "Expected successful analysis but got an error: {:?}",
            result
        );
        let scale_factor = result.unwrap();
        let expected_scale_factor = 10.0; // Adjust based on your scaling logic
        assert_eq!(
            scale_factor, expected_scale_factor,
            "The scale factor did not match the expected value"
        );
    }
}
