use serde::{Deserialize, Serialize};

/// Update type for the dRISK API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub red: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub green: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blue: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_label: Option<bool>,
}

impl std::ops::AddAssign for NodeUpdate {
    fn add_assign(&mut self, other: NodeUpdate) {
        if let Some(label) = other.label {
            self.label = Some(label);
        }
        if let Some(size) = other.size {
            self.size = Some(size);
        }
        if let Some(url) = other.url {
            self.url = Some(url);
        }
        if let Some(red) = other.red {
            self.red = Some(red);
        }
        if let Some(green) = other.green {
            self.green = Some(green);
        }
        if let Some(blue) = other.blue {
            self.blue = Some(blue);
        }
        if let Some(show_label) = other.show_label {
            self.show_label = Some(show_label);
        }
    }
}
