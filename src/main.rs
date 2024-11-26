use tokio;
use log::info;
use log4rs;

mod config;
mod system_initializer;
mod clients;
use crate::system_initializer::SystemInitializer;
use crate::system_initializer::SYSTEM_INITIALIZER;

pub mod message {
    include!(concat!(env!("OUT_DIR"), "\\message.rs"));
}

#[tokio::main]
async fn main() -> Result<(), Box<Box<dyn std::error::Error>>> {

    let _ = log4rs::init_file("log4rs.yaml", Default::default());
    info!("Application started.");

    let initializer = SYSTEM_INITIALIZER.get_or_init(|| async{
        SystemInitializer::new().await.unwrap()
    }).await;
    initializer.init_process().await?;
    
    Ok(())
}