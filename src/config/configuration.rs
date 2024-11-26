use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::fs::File;
use std::error::Error;
use base64::encode;
use serde::Serializer;
use serde_json::to_string;
use lazy_static::lazy_static;

#[derive(Deserialize, Serialize, Debug)]
pub struct Configuration {
    pub base: String,
    pub header: HeaderConfig,
    pub message: Message,
    pub opc: OpcConfig,
    pub num_producers : usize,
    pub num_consumers : usize
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HeaderConfig {
    pub key: String,
    pub username: String,
    pub password: String,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Message {
    pub msg_type: String,
    #[serde(serialize_with = "serialize_filter_as_string")]
    pub filter: Filter,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Filter {
    #[serde(rename = "dateRange")]
    pub date_range: DateRange,
    pub id: i32,
    #[serde(rename = "type")]
    pub r#type: TagType,
    #[serde(rename = "lastModified")]
    pub last_modified: i64,
    #[serde(rename = "assetName")]
    pub asset_name: String,
    #[serde(rename = "startingRow")]
    pub starting_row: i32,
    #[serde(rename = "maxRecordCount")]
    pub max_record_count: i32,
    #[serde(rename = "orderByProperty")]
    pub order_by_property: String,
    pub descending: bool,
    #[serde(rename = "filterDeleted")]
    pub filter_deleted: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DateRange {
    pub id: i32,
    pub duration: i32,
    pub selection: i32,
    #[serde(rename = "fromDate")]
    pub from_date: i64,
    #[serde(rename = "toDate")]
    pub to_date: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TagType {
    pub id: i32,
}


#[derive(Deserialize, Serialize, Debug)]
pub struct OpcConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

impl Configuration {
    pub fn get_base(&self) -> String {
        self.base.to_string()
    }

    pub fn get_message(&self) -> &Message {
        &self.message
    }

    pub fn load_from_file(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let file = File::open(file_path)?;
        let config:Configuration = from_reader(file)?;
        Ok(config)
    }

    pub fn get_header_key(&self) -> &'static str {
        Box::leak(self.header.key.clone().into_boxed_str())
    }

    pub fn get_header_value(&self) -> String {
        let encoded_credentials = encode(format!("{}:{}", self.header.username, self.header.password));
        format!("Basic {}", encoded_credentials)
    }

    pub fn get_opc_url(&self) -> &str {
        &self.opc.url
    }

    pub fn get_opc_username(&self) -> String {
        self.opc.username.to_string()
    }

    pub fn get_opc_password(&self) -> String {
        self.opc.password.to_string()
    }

    pub fn get_num_producers(&self) -> usize {
        self.num_producers
    }

    pub fn get_num_consumers(&self) -> usize {
        self.num_consumers
    }

}
fn serialize_filter_as_string<S>(filter: &Filter, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let json_str = to_string(&filter).map_err(serde::ser::Error::custom)?; 
    serializer.serialize_str(&json_str) 
}
lazy_static! {
   pub static ref CONFIG: Configuration = Configuration::load_from_file("config.json").unwrap();
}