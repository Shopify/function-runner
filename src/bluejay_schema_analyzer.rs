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
}
