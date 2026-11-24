use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct MenuLabels {
    pub cancel:         String,
    pub apply:          String,  
    pub save_and_apply: String,
    pub back:           String,

    pub active:         String,
    pub cloud:          String,
    pub local:          String,

    pub visible:        String,
    pub hidden:         String,

    pub default:        String,
}

impl MenuLabels{
    pub fn get_cloud_label(&self, is_cloud: bool) -> String{
        if is_cloud {
            self.cloud.clone()
        }
        else {
            self.local.clone()
        }
    }
}

impl Default for MenuLabels {
    fn default() -> Self {
        Self {
            // button
            cancel:         "< Cancel >".to_string(),
            apply:          "< Apply >".to_string(),
            save_and_apply: "< Save & Apply >".to_string(),
            back:           "< Back >".to_string(),
            // label
            active:         "[active]".to_string(),
            cloud:          "[cloud]".to_string(),
            local:          "[local]".to_string(),
            // info
            visible:        "(visible)".to_string(),
            hidden:         "(hidden)".to_string(),

            default:        "default".to_string(),
        }
    }
}