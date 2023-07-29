use crate::runtime::Step;
use async_trait::async_trait;
use std::time::Duration;

struct AutoBuy {}

impl AutoBuy {
    fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Step for AutoBuy {
    async fn step(&self) -> Option<Duration> {
        None
    }
}
