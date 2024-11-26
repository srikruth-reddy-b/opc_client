use log::error;
use log::info;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::client_async;
use tokio_native_tls::TlsStream;
use std::error::Error;
use url::Url;
use futures_util::StreamExt;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use serde_json::Value;
use std::thread::sleep;
use std::time::Duration;

pub struct WebSocketClient {
    backend_url: String,
    header_key: &'static str,
    header_value: String,
    ws_client: Option<tokio_tungstenite::WebSocketStream<TlsStream<tokio::net::TcpStream>>>,
}

impl WebSocketClient {
    pub fn new(backend_url: String, header_key: &'static str, header_value: String) -> Self {
        WebSocketClient {
            backend_url,
            header_key,
            header_value,
            ws_client: None,
        }
    }

    pub async fn connect_to_c2_server(&mut self) -> Result<(), Box<dyn Error>> {
        loop{
        let  ws_url: Url = self.backend_url.parse()?;
        let tls_stream = self.open_tls_stream(&ws_url).await?;
        let mut request = ws_url.into_client_request()?;

        let headers = request.headers_mut();
        headers.insert(self.header_key, self.header_value.parse().unwrap());

        match client_async(request, tls_stream).await {
            Ok((ws_client, _)) => {
                self.ws_client = Some(ws_client);
                break;
            }
            Err(err) => {
                error!("Failed to connect to C2 Server: {}", err);
                sleep(Duration::from_secs(5));
            }
        }
    }
        info!("Connected to C2 Server");
        Ok(())
    }

    async fn open_tls_stream(&self, ws_url: &Url) -> Result<TlsStream<tokio::net::TcpStream>, Box<dyn Error>> {
        let connector = tokio_native_tls::native_tls::TlsConnector::builder().build()?;
        let connector: tokio_native_tls::TlsConnector = connector.into();
        let addrs = ws_url.socket_addrs(|| None)?;
        let stream = tokio::net::TcpStream::connect(&*addrs).await?;
        let tls_stream = connector.connect(ws_url.host_str().unwrap(), stream).await?;

        Ok(tls_stream)
    }

    pub async fn send_tag_request(&mut self,tag_request : String) -> Result<(), Box<dyn Error>> {
        let ws_client = self.ws_client.as_mut().ok_or("WebSocket client not initialized")?;    
        ws_client.send(tokio_tungstenite::tungstenite::Message::Text(tag_request)).await?;
        info!("Message sent");
        Ok(())
    }
    pub async fn receive_response(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        let ws_client = self.ws_client.as_mut().ok_or("WebSocket client not initialized")?;
    
        let mut tags_vec = Vec::new();
        while let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(response))) = ws_client.next().await {
            let without_32nd = format!("{}{}", &response[..31], &response[32..]);
            let len = without_32nd.len();
            let modified_string = format!("{}{}", &without_32nd[..len - 2], &without_32nd[len - 1..]);
    
            let outer_response: Value = serde_json::from_str(&modified_string)?;
            if let Some(inner_msg) = outer_response["msg"].as_object() {
                if let Some(data_list) = inner_msg["data"].as_array() {
                    for item in data_list {
                        if let Some(tag_name) = item["tagName"].as_str() {
                            tags_vec.push(tag_name.to_string());
                        }
                    }
                }
                if let Some(final_batch) = inner_msg.get("finalBatch").and_then(Value::as_bool) {
                    if final_batch {
                        println!("Final Batch: {}", final_batch.to_string());
                        break;
                    }
                }
            }
        }
    
        if tags_vec.is_empty() {
            Ok(vec!["Server didn't respond".to_string()])
        } else {
            Ok(tags_vec)
        }
    }

    pub async fn push_to_c2(&mut self,universal_buffer : Vec<u8>)-> Result<(), Box<dyn Error>>{
        //info!("{}",universal_buffer.len());
        let ws_client = self.ws_client.as_mut().ok_or("WebSocket client not initialized")?;
        if let Err(e) = ws_client.send(WsMessage::Binary(universal_buffer)).await {
            error!("Failed to send Universal data: {:?}", e);
        }
        Ok(())
    }
}
