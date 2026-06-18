use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

enum Message {
    Run(Box<dyn FnOnce() + Send + 'static>),
    Quit,
}

struct Inner {
    queue: Mutex<VecDeque<Message>>,
    condvar: Condvar,
}

pub struct ThreadPool {
    inner: Arc<Inner>,
    handles: Vec<JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0, "thread pool size must be > 0");
        let inner = Arc::new(Inner {
            queue: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
        });

        let mut handles = Vec::with_capacity(size);
        for _ in 0..size {
            let inner = Arc::clone(&inner);
            handles.push(thread::spawn(move || loop {
                let job = {
                    let mut queue = inner.queue.lock().unwrap();
                    while queue.is_empty() {
                        queue = inner.condvar.wait(queue).unwrap();
                    }
                    queue.pop_front()
                };
                match job {
                    Some(Message::Run(f)) => {
                        f();
                    }
                    Some(Message::Quit) | None => break,
                }
            }));
        }

        ThreadPool { inner, handles }
    }

    pub fn size(&self) -> usize {
        self.handles.len()
    }

    pub fn execute<F: FnOnce() + Send + 'static>(&self, f: F) {
        {
            let mut queue = self.inner.queue.lock().unwrap();
            queue.push_back(Message::Run(Box::new(f)));
        }
        self.inner.condvar.notify_one();
    }

    pub fn execute_batch<I>(&self, jobs: I)
    where
        I: IntoIterator,
        I::Item: FnOnce() + Send + 'static,
    {
        {
            let mut queue = self.inner.queue.lock().unwrap();
            for job in jobs {
                queue.push_back(Message::Run(Box::new(job)));
            }
        }
        self.inner.condvar.notify_all();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in 0..self.handles.len() {
            {
                let mut queue = self.inner.queue.lock().unwrap();
                queue.push_back(Message::Quit);
            }
            self.inner.condvar.notify_one();
        }
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::search::{search, SearchParams};
    use crate::tt::TT;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn test_pool_multi_threaded_search() {
        crate::attack::init_slider_tables();
        let board = Board::from_initial();
        let mut params = SearchParams::with_depth(4);
        params.threads = 2;
        let stop = Arc::new(AtomicBool::new(false));
        let tt = Arc::new(TT::new(16));
        let pool = ThreadPool::new(4);
        let result = search(&board, &params, &stop, &tt, Some(&pool));
        assert!(result.best_move.is_some(), "pool multi-threaded search should return a move");
    }
}
