use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConverterCategory {
    Document,
    Pdf,
    Image,
    Audio,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConverterStatus {
    Ready,
    ExternalRequired,
    Planned,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConverterCapability {
    pub id: String,
    pub category: ConverterCategory,
    pub title_key: String,
    pub description_key: String,
    pub engine: String,
    pub status: ConverterStatus,
    pub bundled: bool,
    pub input_extensions: Vec<String>,
    pub output_extensions: Vec<String>,
}

impl ConverterCapability {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        category: ConverterCategory,
        title_key: &str,
        description_key: &str,
        engine: &str,
        status: ConverterStatus,
        bundled: bool,
        input_extensions: &[&str],
        output_extensions: &[&str],
    ) -> Self {
        Self {
            id: id.into(),
            category,
            title_key: title_key.into(),
            description_key: description_key.into(),
            engine: engine.into(),
            status,
            bundled,
            input_extensions: input_extensions
                .iter()
                .map(|value| (*value).into())
                .collect(),
            output_extensions: output_extensions
                .iter()
                .map(|value| (*value).into())
                .collect(),
        }
    }
}
