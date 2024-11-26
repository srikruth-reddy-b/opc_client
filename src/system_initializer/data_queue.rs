use lazy_static::lazy_static;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::{ Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use log::info;

pub struct Queue {
    buffer: Mutex<VecDeque<Vec<u8>>>,
    empty_slots: Condvar,
    full_slots: Condvar,
    pub consumed_count: AtomicUsize,
    pub produced_count: AtomicUsize,
    max_size: usize,
}

impl Queue {
    pub fn new(data_queue_size:usize) -> Self {
        Queue {
            buffer: Mutex::new(VecDeque::with_capacity(data_queue_size)),
            empty_slots: Condvar::new(),
            full_slots: Condvar::new(),
            consumed_count: AtomicUsize::new(0),
            produced_count: AtomicUsize::new(0),
            max_size: data_queue_size,
        }
    }

    pub fn enqueue(&self, item: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let mut buffer: std::sync::MutexGuard<'_, VecDeque<Vec<u8>>> = self.buffer.lock().unwrap();
        while buffer.len() == self.max_size {
            info!("Buffer is full. Producer is waiting...");
            buffer = self.full_slots.wait(buffer).unwrap();
        }
        buffer.push_back(item);
        self.produced_count.fetch_add(1, Ordering::SeqCst);
        self.empty_slots.notify_one();
        
        Ok(())
    }
    
    pub fn dequeue(&self) -> Vec<u8> {
        let mut buffer = self.buffer.lock().unwrap();
        while buffer.is_empty() {
            info!("Buffer is empty. Consumer is waiting...");
            buffer = self.empty_slots.wait(buffer).unwrap();
        }
        let item = buffer.pop_front().unwrap_or_default();
        self.full_slots.notify_one();
        self.consumed_count.fetch_add(1, Ordering::SeqCst);
        item
    }

    pub fn get_consumed_count(&self) -> usize {
        self.consumed_count.load(Ordering::SeqCst)
    }
    pub fn get_produced_count(&self) -> usize {
        self.produced_count.load(Ordering::SeqCst)
    }
}
lazy_static! {
    pub static ref QUEUE: Arc<Queue> = Arc::new(Queue::new(500000));
}
