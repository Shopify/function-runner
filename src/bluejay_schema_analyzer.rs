use bluejay_parser::error::{Annotation, Error as BluejayError};
use bluejay_core::definition::ObjectTypeDefinition;
use bluejay_core::definition::SchemaDefinition as CoreSchemaDefinition;
use bluejay_core::AsIter;
use bluejay_core::Directive;
use bluejay_core::Value;
use bluejay_core::{definition::HasDirectives, ValueReference};
use bluejay_parser::{
    ast::{
        definition::FieldDefinition,
        definition::{DefaultContext, DefinitionDocument, SchemaDefinition},
        executable::ExecutableDocument,
        Parse,
    },
    Error,
};
use crate::scale_limits_analyzer::ScaleLimitsAnalyzer;

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

    pub fn analyze_schema_definition(schema_definition: SchemaDefinition, query: &str) {
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
    }
}
