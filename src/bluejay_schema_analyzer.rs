use crate::scale_limits_analyzer::ScaleLimitsAnalyzer;
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
    pub fn analyze_schema_definition(
        schema_string: &str,
        query: &str,
        input: &serde_json::Value,
    ) -> Result<f64, Error> {
        let document_definition = DefinitionDocument::parse(schema_string).unwrap();

        let schema_definition_result =
            SchemaDefinition::try_from(&document_definition).map_err(|errors| {
                errors
                    .into_iter()
                    .map(|e| Error::new(format!("Invalid Schema: {e:?}"), None, Vec::new()))
                    .collect::<Vec<Error>>() // Explicit type annotation here
            });

        let scale_factor = match schema_definition_result {
            Ok(schema_definition) => {
                let executable_document = ExecutableDocument::parse(query);

                match executable_document {
                    Ok(ed) => {
                        let cache =
                            bluejay_validator::executable::Cache::new(&ed, &schema_definition);

                        let result = ScaleLimitsAnalyzer::analyze(
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
                        });

                        match result {
                            Ok(scale_factor) => scale_factor,
                            Err(err) => {
                                eprintln!("{:?}", err.message());
                                1.0
                            }
                        }
                    }
                    Err(_e) => {
                        eprintln!("Error creating the executable document");
                        1.0
                    }
                }
            }
            Err(errors) => {
                for error in errors {
                    eprintln!("Error creating schema definition: {:?}", error);
                }
                1.0
            }
        };

        Ok(scale_factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_create_definition_document_valid() {
    //     let schema = r#"
    //     type Query {
    //         field: String @scaleLimits(rate: 1.5)
    //     }
    //     "#;
    //     let result = BluejaySchemaAnalyzer::create_definition_document(schema);
    //     assert!(result.is_ok());
    // }

    // #[test]
    // fn test_create_definition_document_invalid() {
    //     let schema = "type Query { field: String";
    //     let result = BluejaySchemaAnalyzer::create_definition_document(schema);
    //     assert!(result.is_err());
    // }

    // #[test]
    // fn test_create_schema_definition_valid() {
    //     let schema_string = r#"
    //         directive @scaleLimits(rate: Float!) on FIELD_DEFINITION

    //         type Query {
    //             field: String @scaleLimits(rate: 0.005)
    //         }
    //     "#;
    //     let definition_document = BluejaySchemaAnalyzer::create_definition_document(schema_string)
    //         .expect("Schema had parse errors");
    //     let result = BluejaySchemaAnalyzer::create_schema_definition(&definition_document);

    //     assert!(result.is_ok());
    // }

    // #[test]
    // fn test_create_schema_definition_invalid() {
    //     let schema_string = r#"
    //         type Query {
    //             field: String @scaleLimits(rate: "invalid_rate")
    //         }
    //     "#;
    //     let definition_document =
    //         BluejaySchemaAnalyzer::create_definition_document(schema_string).unwrap();
    //     let result = BluejaySchemaAnalyzer::create_schema_definition(&definition_document);
    //     assert!(result.is_err());
    // }

    // #[test]
    // fn test_analyze_schema_definition() {
    //     let schema_string = r#"
    //         directive @scaleLimits(rate: Float!) on FIELD_DEFINITION

    //         type Query {
    //             field: String @scaleLimits(rate: 0.005)
    //         }
    //     "#;
    //     let query = "{ field }";
    //     let input_json = serde_json::json!({
    //         "field": "value"
    //     });

    //     let definition_document =
    //         BluejaySchemaAnalyzer::create_definition_document(schema_string).unwrap();
    //     let schema_definition =
    //         BluejaySchemaAnalyzer::create_schema_definition(&definition_document).unwrap();
    //     let result =
    //         BluejaySchemaAnalyzer::analyze_schema_definition(schema_definition, query, &input_json);

    //     assert!(
    //         result.is_ok(),
    //         "Expected successful analysis but got an error: {:?}",
    //         result
    //     );

    //     let scale_factor = result.unwrap();
    //     let expected_scale_factor = 1.0; // This should be the expected result based on your application logic
    //     assert_eq!(
    //         scale_factor, expected_scale_factor,
    //         "The scale factor did not match the expected value"
    //     );
    // }

    // #[test]
    // fn test_analyze_schema_with_large_input() {
    //     let schema_string = r#"
    //         directive @scaleLimits(rate: Float!) on FIELD_DEFINITION

    //         type Query {
    //             field: [String] @scaleLimits(rate: 0.005)
    //         }
    //     "#;
    //     let query = "{ field }";
    //     let input_json = serde_json::json!({
    //       "field": vec!["value"; 10000]
    //     });

    //     let definition_document =
    //         BluejaySchemaAnalyzer::create_definition_document(schema_string).unwrap();
    //     let schema_definition =
    //         BluejaySchemaAnalyzer::create_schema_definition(&definition_document).unwrap();
    //     let result =
    //         BluejaySchemaAnalyzer::analyze_schema_definition(schema_definition, query, &input_json);

    //     assert!(
    //         result.is_ok(),
    //         "Expected successful analysis but got an error: {:?}",
    //         result
    //     );
    //     let scale_factor = result.unwrap();
    //     let expected_scale_factor = 10.0; // Adjust based on your scaling logic
    //     assert_eq!(
    //         scale_factor, expected_scale_factor,
    //         "The scale factor did not match the expected value"
    //     );
    // }
}
