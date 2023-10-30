//! Thread Pool
//! The implementation is taken from the [book](https://doc.rust-lang.org/book/ch20-02-multithreaded.html)

use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            if log::log_enabled!(log::Level::Trace) {
                log::trace!("Shutting down worker {}", worker.id);
            }
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, receiver.clone()));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }

    pub fn join(&mut self) {
        unimplemented!()
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    if log::log_enabled!(log::Level::Trace) {
                        log::trace!("Worker {id} got a job; executing.");
                    }

                    job();
                }
                Err(_) => {
                    if log::log_enabled!(log::Level::Trace) {
                        log::trace!("Worker {id} disconnected; shutting down.");
                    }
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

#[cfg(test)]
pub mod test {

    use std::time::Duration;

    use super::*;

    #[test]
    fn thread_pool_test() {
        use std::sync::atomic::{AtomicU64, Ordering};

        let total = Arc::new(AtomicU64::new(0));
        let tasks: Vec<_> = (0..100).collect();
        let job_task = |n: u64| {thread::sleep(Duration::from_millis(20)); n*n};
        let pool = ThreadPool::new(4);

        for task in tasks {
            let total_clone = total.clone();
            pool.execute(move || {
                let product = job_task(task);
                total_clone.fetch_add(product, Ordering::SeqCst);
            });
        }

        // wait for executed threads complete
        // pool.join();
        drop(pool);

        assert_eq!(total.load(Ordering::SeqCst), 328350);
    }
}
