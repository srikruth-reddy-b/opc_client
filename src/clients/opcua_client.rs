use std::sync::Arc;
use opcua::sync::RwLock;
use tokio;
use prost::Message;
use opcua::client::prelude::*;
use log::{info, error};
use opcua::types::Variant::Float;
use chrono::{DateTime, Utc};

use crate::config::configuration::CONFIG;
use crate::message::{Historical, HistoricalValue};
use crate::system_initializer::data_queue::QUEUE;

pub struct OpcuaClient {
    pub session: Option<Arc<RwLock<Session>>>,
}

impl OpcuaClient {
    pub fn new() -> Self {
        OpcuaClient {
            session: None,
        }
    }

    pub fn connect_to_opcserver(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    
        let kep_url = CONFIG.get_opc_url();
        let kep_user = CONFIG.get_opc_username();
        let kep_password = CONFIG.get_opc_password();

        let _ = tokio::task::block_in_place(|| -> Result<(), Box<dyn std::error::Error>> {
            let mut client: Client = ClientBuilder::new()
                .application_name("Client1")
                .application_uri("urn:client1")
                .product_uri("urn:client11")
                .trust_server_certs(false)
                .create_sample_keypair(true)
                .session_retry_limit(3)
                .client()
                .unwrap();

            if let Ok(session) = client.connect_to_endpoint(
                (
                    kep_url,
                    SecurityPolicy::None.to_str(),
                    MessageSecurityMode::None,
                    UserTokenPolicy::anonymous(),
                ),
                IdentityToken::UserName(kep_user, kep_password),
            ) {
                self.session = Some(session);
            }
            else {
                error!("Failed to connect to OPC UA endpoint.");
                return Err("Failed to connect to OPC UA endpoint.".into());
            }
            Ok(())
        })?;

        Ok(())
    }

    pub fn browse_nodes(&self, node_id: NodeId, tags_vec: &[String]) -> Result<Vec<String>, StatusCode> {
        let session = self.session.as_ref().unwrap().read();
        let session = &*session;
        let mut matched_tags = Vec::new();
        let browse_description = BrowseDescription {
            node_id: node_id.clone(),
            browse_direction: BrowseDirection::Forward,
            reference_type_id: ReferenceTypeId::Organizes.into(),
            include_subtypes: true,
            node_class_mask: 0,
            result_mask: BrowseDescriptionResultMask::all().bits() as u32,
        };
        
        let results = session.browse(&[browse_description]).map_err(|e| {
            error!("Failed to browse node: {}", e);
            e
        })?;

        if let Some(results_vec) = results {
            for result in results_vec.iter() {
                if let Some(references) = &result.references {
                    for reference in references {
                        let child_node_id = reference.node_id.node_id.clone();
                        let device_names: Vec<String> = (1..=102).map(|i| format!("Device{}", i)).collect();
                        // if reference.display_name.text.to_string().contains("Device5000") {
                        if device_names.iter().any(|device| reference.display_name.text.to_string().contains(device)) {
                            let device_tags = self.browse_tags(session, child_node_id, tags_vec)?;
                            matched_tags.extend(device_tags);
                        } else {
                            let child_matched_tags = self.browse_nodes(child_node_id, tags_vec)?;
                            matched_tags.extend(child_matched_tags);
                        }
                    }
                }
            }
        }

      Ok(matched_tags)
    }

    fn browse_tags(&self, session: &Session, channel_node_id: NodeId, tags_vec: &[String]) -> Result<Vec<String>, StatusCode> {
        let mut matched_tags = Vec::new();
        let browse_description = BrowseDescription {
            node_id: channel_node_id.clone(),
            browse_direction: BrowseDirection::Forward,
            reference_type_id: ReferenceTypeId::HasComponent.into(),
            include_subtypes: true,
            node_class_mask: 0,
            result_mask: BrowseDescriptionResultMask::all().bits() as u32,
        };

        let results = session.browse(&[browse_description])?;
        if let Some(results_vec) = results {
            for result in results_vec.iter() {
                if let Some(references) = &result.references {
                    for reference in references {
                        let tag_name = reference.display_name.text.clone();
                        let node_id = reference.node_id.clone().to_string();                   
                        let result = &node_id[13..];
                        if tags_vec.contains(&tag_name.to_string()) {
                            matched_tags.push(result.to_string());
                        }
                    }
                }
            }
        }
      Ok(matched_tags)
    }

    pub async fn subscribe_tags(&self, tags_vec: Vec<String>, ns: u16) -> Result<(), StatusCode> {
        if let Some(session) = self.session.as_ref() {
            let session = session.read();
            let subscription_id = session.create_subscription(
                1000.0,
                30*3,
                30,
                0,
                0,
                true,
                DataChangeCallback::new( move |changed_monitored_items| {
                for item in changed_monitored_items {
                    let historical_buffer = Self::process_historical(item);
                    tokio::spawn(async move {
                       if let Err(_) =  QUEUE.enqueue(historical_buffer){
                            error!("Error sending data to DataQueue");
                       }
                    }); 
                }
            }),
        ).map_err(|e| {
            error!("Failed to create subscription: {}", e);
            e
        })?;
            info!("Created a subscription with id = {}", subscription_id);
            const BATCH_SIZE: usize = 809;
            for chunk in tags_vec.chunks(BATCH_SIZE) {
                let items_to_create: Vec<MonitoredItemCreateRequest> = chunk
                    .iter()
                    .map(|v| NodeId::new(ns, v.clone()).into())
                    .collect();
                let _ = session.create_monitored_items(subscription_id, TimestampsToReturn::Both, &items_to_create)?;
            }
        }
        else {
            error!("Session is not available for subscribing to variables.");
            return Err(StatusCode::BadSessionIdInvalid);
        }
        Ok(())
    }
    
    fn process_historical(item: &MonitoredItem)-> Vec<u8> {        
        let node_id = &item.item_to_monitor().node_id;
        let data_value = item.last_value();

        let mut value: f64 = 0.0; 
        let mut timestamp_millis: Option<i64> = None;

        if let Some(ref value_) = data_value.value {
            if let Float(value_float) = value_ {
                value = *value_float as f64; 
            }

            if let Some(timestamp) = &data_value.source_timestamp {
                if let Some(ts_millis) = Self::datetime_to_timestamp_millis(&timestamp.to_string()) {
                    timestamp_millis = Some(ts_millis); 
                }
            }
        }
        if let (value, Some(timestamp_millis)) = (value, timestamp_millis) {

            let mut tag_name = "".to_string();
            match Self::extract_tagname(node_id.to_string()) {
                Some(tagname) => tag_name = tagname,
                None => println!("No tag found."),
            }
            //println!("{}",tag_name);
            let historical_data = Historical {
                batchid: 1000,
                sensor: tag_name,
                values: vec![HistoricalValue { t: timestamp_millis, v: value }],
            };
            let mut historical_buffer = Vec::new();
            if historical_data.encode(&mut historical_buffer).is_err() {
                error!("Error encoding historical data");
                return historical_buffer;
            }
        return historical_buffer;
        } else {
            info!("No valid value or timestamp available for historical data.");
            return vec![1];
        }  
    }        

    fn datetime_to_timestamp_millis(datetime_str: &str) -> Option<i64> {
        if let Ok(datetime) = DateTime::parse_from_rfc3339(datetime_str) {
            let timestamp_millis = datetime.with_timezone(&Utc).timestamp_millis();
            Some(timestamp_millis)
        } else {
            None
        }
    }
    fn extract_tagname(s: String) -> Option<String> {
        let parts: Vec<&str> = s.split('.').collect();
        parts.last().map(|&tag| tag.to_string())
    }
}
