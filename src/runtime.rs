use async_trait::async_trait;

use futures::stream::FuturesUnordered;

use futures::StreamExt as _;
use log::{debug, error};
///
/// Runtime is a simple runtime for executing steps repeatedly in parallel
/// We get control over the concurrency and the priority of each step
///
use priority_queue::PriorityQueue;

use std::{
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    time::Instant,
};

pub struct Runtime {
    items: Vec<Arc<Mutex<RuntimeItem>>>,
    concurrency: i64,
    prequeue: RwLock<PriorityQueue<(usize, i64), Instant>>,
    queue: RwLock<PriorityQueue<usize, i64>>,

    num_running: AtomicI64,
    sender: UnboundedSender<usize>,
    recv: Mutex<UnboundedReceiver<usize>>,
}

struct RuntimeItem {
    step: Box<dyn Step + Send + Sync>,
    priority: i64,
}

impl Runtime {
    pub fn new(concurrency: i64) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            items: vec![],
            concurrency,
            prequeue: RwLock::new(PriorityQueue::new()),
            queue: RwLock::new(PriorityQueue::new()),
            num_running: AtomicI64::new(0),
            sender: tx,
            recv: Mutex::new(rx),
        }
    }

    pub async fn add(&mut self, step: Box<dyn Step + Send + Sync>, priority: i64) {
        let idx = self.items.len();
        self.items
            .push(Arc::new(Mutex::new(RuntimeItem { step, priority })));
        self.queue.write().await.push(idx, priority);
    }

    pub async fn run(&self) {
        let mut rx = self.recv.lock().await;
        let mut futures = FuturesUnordered::new();

        self.try_dequeue().await;

        loop {
            let next_prequeue_instant = (self.prequeue.read().await.peek())
                .map(|(_, instant)| *instant)
                .unwrap_or(Instant::now() + Duration::from_secs(3600));
            tokio::select! {
                Some(idx) = rx.recv() => {
                    let item = self.items[idx].clone();
                    let run_step = async move {
                        let start = Instant::now();
                        let ret = item.lock().await.step.step().await;
                        let end = Instant::now();
                        debug!("Step {} took {}ms", idx, (end - start).as_millis());
                        (idx, ret)
                    };
                    let fut = async move {
                        let join_result = tokio::spawn(run_step).await;
                        (idx, join_result)
                    };
                    futures.push(fut);
                },
                Some((_idx, join_result)) = futures.next() => {
                    // log::debug!("{:?}", join_result);
                    self.num_running.fetch_add(-1, Ordering::SeqCst);
                    match join_result {
                        Ok(result) => {
                            let (idx, ret) = result;
                            if let Some(duration) = ret {
                                let priority = self.items[idx].lock().await.priority;
                                let instant = tokio::time::Instant::now() + duration;
                                self.prequeue.write().await.push((idx, priority), instant);
                            }
                        },
                        Err(e) => {
                            error!("Join error in step: {:?}", e);
                        }
                    }
                    self.try_dequeue().await;
                },
                _ = tokio::time::sleep_until(next_prequeue_instant) => {
                    self.try_dequeue().await;
                },
                else => break,
            }
        }
    }

    async fn try_dequeue(&self) {
        let mut prequeue = self.prequeue.write().await;
        let mut queue = self.queue.write().await;

        loop {
            let mut added = false;
            let front = prequeue.peek().map(|(idx, instant)| (*idx, *instant));
            if let Some(((idx, priority), instant)) = front {
                if instant <= tokio::time::Instant::now() {
                    prequeue.pop();
                    queue.push(idx, priority);
                    added = true;
                }
            }
            if !added {
                break;
            }
        }
        drop(prequeue);
        drop(queue);

        while self.num_running.load(Ordering::SeqCst) < self.concurrency {
            let front = {
                let mut queue = self.queue.write().await;

                queue.pop()
            };
            if let Some((queue_idx, _priority)) = front {
                let running_prev = self.num_running.fetch_add(1, Ordering::SeqCst);
                self.sender.send(queue_idx).unwrap();
                if running_prev + 1 >= self.concurrency {
                    return;
                }
            } else {
                return;
            }
        }
    }
}

type StepResult = Option<Duration>;

#[async_trait]
pub trait Step {
    async fn step(&self) -> StepResult;
}

#[cfg(test)]
mod test {
    use super::*;

    struct TestExecutor(Arc<Mutex<i64>>);
    impl TestExecutor {
        fn new(x: i64) -> Self {
            Self(Arc::new(Mutex::new(x)))
        }
    }
    #[async_trait]
    impl Step for TestExecutor {
        async fn step(&self) -> StepResult {
            let mut x = self.0.lock().await;
            *x -= 1;
            println!("step: {}", x);
            if *x == 0 {
                None
            } else {
                Some(Duration::from_secs(0))
            }
        }
    }

    #[tokio::test]
    async fn test() {
        let mut runtime = Runtime::new();
        runtime.add(Box::new(TestExecutor::new(3)), 0).await;
        runtime.add(Box::new(TestExecutor::new(4)), 0).await;
        runtime.run().await;
    }
}
