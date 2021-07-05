use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct OnlyId {
    pub id: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Element {
    pub id: usize,
    pub name: String,
    pub value: serde_json::Value,
}

#[allow(dead_code)]
impl Element {
    pub fn new(id: usize, name: String, value: String) -> Element {
        Element {
            id,
            name,
            value: serde_json::from_str(&value).unwrap(),
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_value(&mut self, value: String) {
        self.value = serde_json::from_str(&value).unwrap();
    }
}
impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Id: {}, Name: {}, Value: {}",
            self.id, self.name, self.value
        )
    }
}
