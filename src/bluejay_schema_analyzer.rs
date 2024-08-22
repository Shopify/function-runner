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
                .map(|e| {
                    BluejayError::new(
                        "something went wrong?!", // Error message
                        None,
                        Vec::new(),
                    )
                })
                .collect()
        })
    }

    pub fn analyze_schema_definition(
        schema_definition: SchemaDefinition,
        query: &str,
    ) -> Result<f64, Error> {
        let executable_document =
            ExecutableDocument::parse(query).expect("Document had parse errors");
        let cache =
            bluejay_validator::executable::Cache::new(&executable_document, &schema_definition);

        let scale_factor = ScaleLimitsAnalyzer::analyze(
            &executable_document,
            &schema_definition,
            None,
            &Default::default(),
            &cache,
        )
        .expect("Analysis failed");

        eprintln!(
            "Success creating ed, pass thing into analyzer? {:?}",
            scale_factor
        );

        Ok(scale_factor)
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
