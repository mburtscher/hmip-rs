use serde::{Deserialize, Serialize};

use super::types::PropertyType;

/// Describes a group of configuration properties that HCUweb presents together.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupTemplate {
    /// A human readable label.
    pub friendly_name: String,
    /// A human readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Position in HCUweb, lowest first. Orders need not be sequential.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
}

/// Describes a single configuration property shown in HCUweb.
///
/// Which constraint fields apply depends on [`data_type`](Self::data_type): `minimum`/`maximum`
/// for `INTEGER` and `NUMBER`, `minimum_length`/`maximum_length`/`pattern` for `STRING` and
/// `PASSWORD`, `values` for `ENUM` and `TYPEAHEAD`; `READONLY`, `QRCODE` and `WEBLINK` use
/// `current_value` (and `default_value` as info text for links).
///
/// ```
/// use hmip_rs::domain::{PropertyTemplate, types::PropertyType};
///
/// let mut interval = PropertyTemplate::new(PropertyType::Integer, "Polling interval (s)");
/// interval.minimum = Some(5);
/// interval.maximum = Some(3600);
/// interval.default_value = Some("30".into());
/// interval.required = Some(true);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyTemplate {
    /// Data type of the property.
    pub data_type: PropertyType,
    /// A human readable label.
    pub friendly_name: String,
    /// The currently configured value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
    /// A default value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    /// A human readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Identifier of the group HCUweb shows this property in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    /// Maximum value for numeric properties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i32>,
    /// Maximum length for text based properties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_length: Option<i32>,
    /// Minimum value for numeric properties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i32>,
    /// Minimum length for text based properties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_length: Option<i32>,
    /// Position in HCUweb, lowest first. Orders need not be sequential.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    /// Regular expression (ECMAScript syntax) text based properties must match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Whether the property is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// Allowed values for `ENUM` and `TYPEAHEAD` properties.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

impl PropertyTemplate {
    /// Creates a property template of the given type and label; everything else is unset.
    pub fn new(data_type: PropertyType, friendly_name: impl Into<String>) -> Self {
        Self {
            data_type,
            friendly_name: friendly_name.into(),
            current_value: None,
            default_value: None,
            description: None,
            group_id: None,
            maximum: None,
            maximum_length: None,
            minimum: None,
            minimum_length: None,
            order: None,
            pattern: None,
            required: None,
            values: None,
        }
    }
}

impl Default for PropertyTemplate {
    fn default() -> Self {
        Self::new(PropertyType::String, String::new())
    }
}
