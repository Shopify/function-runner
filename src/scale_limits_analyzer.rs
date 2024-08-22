
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
    },
    Error,
};

pub type ScaleLimitsAnalyzer<'a> = bluejay_validator::executable::operation::Orchestrator<
    'a,
    ExecutableDocument<'a>,
    SchemaDefinition<'a>,
    serde_json::Map<String, serde_json::Value>,
    ScaleLimits,
>;

pub struct ScaleLimits {
  max_rate: f64,
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
        field_definition.directives().and_then(|directives| {
            directives
                .iter()
                .find(|directive| directive.name() == "scaleLimits")
                .and_then(|directive| {
                    directive.arguments()
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
                                rate
                            } else {
                                None
                            }
                        })
                })
        });
    }
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
