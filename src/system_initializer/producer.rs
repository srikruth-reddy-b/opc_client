use tokio;
use log::{info, error};
use crate::clients::opcua_client::OpcuaClient;
use opcua::types::ObjectId;
use std::error::Error;
use opcua::client::prelude::*;

pub struct Producer {
    opcua_client: OpcuaClient,
}

impl Producer {
    pub fn new() -> Self {
        let opcua_client = OpcuaClient::new();
        Producer { opcua_client }
    }

    pub async fn produce(&mut self, tags: Vec<String>) -> Result<(), Box<dyn Error>> {
        if let Err(e) = self.opcua_client.connect_to_opcserver() {
            error!("Failed to connect: {}", e);
            return Ok(());
        }
        info!("Connected to the OPC UA server successfully.");

        let root_node_id = ObjectId::RootFolder.into();
        let matched_tags = match self.opcua_client.browse_nodes(root_node_id, &tags) {
            Ok(tags) => {
                info!("Matched tags count: {}", tags.len());
                tags
            }
            Err(e) => {
                error!("Error browsing: {}", e);
                return Ok(());
            }
        };

        let batch_size = 1000;
        let total_tags = matched_tags.len();
        let mut start = 0;
        while start < total_tags {
            let end = (start + batch_size).min(total_tags);
            let tag_batch = matched_tags[start..end].to_vec();
            if let Err(err) = self.opcua_client.subscribe_tags(tag_batch.clone(), 2).await {
                error!("Failed to subscribe to variables: {}", err);
            } else {
                //info!("Subscribed");
            }
            start = end;
        }
        if let Some(session) = self.opcua_client.session.clone() {
            tokio::task::spawn_blocking(move || {
                Session::run(session);
            });
        } else {
            error!("Session is not available.");
        }

        Ok(())
    }
}
