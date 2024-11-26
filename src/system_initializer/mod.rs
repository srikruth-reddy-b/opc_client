pub mod producer;
pub mod consumer;
pub mod data_queue;
pub mod tags_synchronizer;

use log::{info, error};
use tokio;
use tokio::time::{sleep, Duration};
use std::sync::atomic::Ordering;
use tokio::sync::OnceCell;
use crate::system_initializer::tags_synchronizer::TagSynchronizer;
use crate::config::configuration::CONFIG;
use crate::clients::ws_client::WebSocketClient;
use crate::system_initializer::data_queue::QUEUE;
pub struct SystemInitializer {
    tags_vec: Vec<String>,
    num_producers: usize,
    num_consumers: usize,
}

impl SystemInitializer {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let tags_vec = Self::sync_tags().await?;
        let num_producers = CONFIG.get_num_producers();
        let num_consumers = CONFIG.get_num_consumers();

        Ok(Self {
            tags_vec,
            num_producers,
            num_consumers,
        })
    }

    pub async fn init_process(&self) -> Result<(), Box<dyn std::error::Error>> {
        let producer_handles = self.init_producer().await?;
        let consumer_handles = self.init_consumer().await?;
        self.log_metrics().await;

        for handle in producer_handles {
            let _ = handle.await;
        }

        for handle in consumer_handles {
            let _ = handle.await;
        }
        Ok(())
    }
    async fn sync_tags() -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut get_tags_obj = TagSynchronizer::new()?;
        let tags_vec = get_tags_obj.get_tags().await?;
        Ok(tags_vec)
    }
    async fn init_producer(&self) -> Result<Vec<tokio::task::JoinHandle<()>>, Box<dyn std::error::Error>> {
        let tags_per_producer = self.tags_vec.len() / self.num_producers;
        let mut producer_handles = Vec::new();

        for (i, producer_tags) in self.tags_vec.chunks(tags_per_producer).enumerate() {
            let producer_tags = producer_tags.to_vec();
            let handle = tokio::spawn(async move {
                let mut producer = producer::Producer::new();
                info!("Producer {} starting with {} tags", i + 1, producer_tags.len());
                if let Err(e) = producer.produce(producer_tags).await {
                    error!("Producer {} encountered an error: {}", i + 1, e);
                }
            });
            producer_handles.push(handle);
        }

        Ok(producer_handles)
    }

    async fn init_consumer(&self) -> Result<Vec<tokio::task::JoinHandle<()>>, Box<dyn std::error::Error>> {
        let mut consumer_handles = Vec::new();
        for i in 0..self.num_consumers {
            let handle = tokio::spawn(async move {
                let mut consumer_obj = consumer::Consumer::new(200);
                info!("Consumer {} starting", i + 1);
                if let Err(e) = consumer_obj.consume().await {
                    error!("Consumer {} encountered an error: {}", i + 1, e);
                }
            });
            consumer_handles.push(handle);
        }

        Ok(consumer_handles)
    }

    async fn log_metrics(&self) {
        let metrics_handle = tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(10)).await;
                let consumed_count = QUEUE.get_consumed_count();
                let produced_count = QUEUE.get_produced_count();
                info!("In the last 10 seconds: Produced: {}, Consumed: {}", produced_count, consumed_count);
                info!("Average: {}", (consumed_count / 10));
                QUEUE.consumed_count.store(0, Ordering::SeqCst);
                QUEUE.produced_count.store(0, Ordering::SeqCst);
            }
        });

        let _ = metrics_handle.await;
    }   
}
pub static SYSTEM_INITIALIZER: OnceCell<SystemInitializer> = OnceCell::const_new();