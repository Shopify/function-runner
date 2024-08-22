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

type ScaleLimitsAnalyzer<'a> = bluejay_validator::executable::operation::Orchestrator<
    'a,
    ExecutableDocument<'a>,
    SchemaDefinition<'a>,
    serde_json::Map<String, serde_json::Value>,
    ScaleLimits,
>;

pub struct BluejaySchemaAnalyzer;

pub struct ScaleLimits {
    max_rate: f64,
}

impl ScaleLimits {
    fn new() -> Self {
        Self { max_rate: 0.0 }
    }

    fn update_max_rate(&mut self, rate: f64) {
        if rate > self.max_rate {
            self.max_rate = rate;
        }
    }

    fn get_max_rate(&self) -> f64 {
        self.max_rate
    }
}

impl
    bluejay_validator::executable::operation::Visitor<
        '_,
        ExecutableDocument<'_>,
        SchemaDefinition<'_>,
        serde_json::Map<String, serde_json::Value>,
    > for ScaleLimits
{
    fn new(
        operation_definition: &'_ <ExecutableDocument as bluejay_core::executable::ExecutableDocument>::OperationDefinition,
        schema_definition: &'_ SchemaDefinition,
        variable_values: &'_ serde_json::Map<String, serde_json::Value>,
        cache: &'_ bluejay_validator::executable::Cache<'_, ExecutableDocument, SchemaDefinition>,
    ) -> Self {
        Self { max_rate: 0.0 }
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
        // println!("field => {:?}", field_definition);
        // println!("field =>");

        field_definition.directives().and_then(|directives| {
            directives
                .iter()
                .find(|directive| directive.name() == "scaleLimits")
                .and_then(|directive| {
                    let x = directive.arguments();

                    eprintln!("errr me now {:?}", x);

                    x
                })
                .and_then(|arguments| {
                    arguments
                        .iter()
                        .find(|argument| argument.name() == "rate")
                        .and_then(|argument| {
                            eprintln!("MEOW CHOW argument {:?}", argument);
                            let value = argument.value();

                            eprintln!("hello value for scaleFactor.rate => {:?}", value);
                            if let ValueReference::Float(rate) = argument.value().as_ref() {
                                self.update_max_rate(rate);
                                let rate = Some(rate);
                                eprintln!("rate? = {:?}", rate);

                                rate
                            } else {
                                None
                            }
                        })
                })
        });
    }
}

impl
    bluejay_validator::executable::operation::Analyzer<
        '_,
        ExecutableDocument<'_>,
        SchemaDefinition<'_>,
        serde_json::Map<String, serde_json::Value>,
    > for ScaleLimits
{
    type Output = f64;

    fn into_output(self) -> Self::Output {
        self.max_rate
    }
}

use bluejay_parser::error::{Annotation, Error as BluejayError};

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
                    // Assuming BluejayError requires a message, an optional annotation, and a vector of annotations
                    BluejayError::new(
                        "something went wrong?!", // Error message
                        None,                     // No specific annotation provided
                        Vec::new(),               // No additional annotations
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
