// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use rayon::ThreadPool;

use crate::sync::{
    Arc,
    Condvar,
    Mutex,
};

/// A multi-threaded work queue that completes when all tasks are done.
///
/// Any thread is able to add tasks regardless if the queue is being worked or not.
pub struct WorkQueue<Task: Send> {
    pool: Arc<ThreadPool>,
    data: Mutex<WorkQueueData<Task>>,
    cond: Condvar,
}
impl<Task: Send> WorkQueue<Task> {
    /// Creates a new work queue with an empty task list.
    pub fn new(pool: &Arc<ThreadPool>) -> Self {
        WorkQueue {
            pool: pool.clone(),
            data: Mutex::new(WorkQueueData::default()),
            cond: Condvar::new(),
        }
    }

    /// Adds a task to the queue while the work queue isn't working.
    pub fn add_task_mut(&mut self, task: Task) {
        self.data.get_mut().tasks.push(task);
    }

    /// Adds all the tasks in the iterator to the queue while the queue isn't working.
    pub fn add_tasks_mut<TaskList>(&mut self, task_list: TaskList)
    where TaskList: Iterator<Item = Task> {
        let queue_data = self.data.get_mut();
        for task in task_list {
            queue_data.tasks.push(task);
        }
    }

    /// Adds a task to the queue regardless if the queue is or is not working.
    /// If another thread was waiting for a task, this will wake up that thread to process it.
    pub fn add_task(&self, task: Task) {
        let mut data = self.data.lock();
        data.tasks.push(task);
        drop(data);
        self.cond.notify_one();
    }

    /// Causes all the threads in the pool to work till there are no more tasks left.
    /// Each task will be given to the per_task function.
    /// # Panics
    /// Panics if the work queue is already working or this thread is from the thread pool.
    /// # Deadlocks
    /// This function only completes once *all* threads in the pool are waiting for a new task.
    /// As such, if a thread gets stuck asleep, this function will never complete.
    ///
    /// This can occur if two WorkQueues are used on the same thread-pool.
    pub fn work<TaskFunc>(&self, per_task: &TaskFunc)
    where TaskFunc: Fn(Task) + Sync {
        {
            let mut data = self.data.lock();
            if data.has_shutdown {
                data.has_shutdown = false;
            } else {
                panic!("WorkQueue cannot be started while it is already running!");
            }
        }
        if self.pool.current_thread_index().is_some() {
            panic!("WorkQueue can not be started by a thread in the thread pool.");
        }
        self.pool.scope(|s| {
            for _ in 0..self.pool.current_num_threads() {
                s.spawn(|_| self.thread_work(per_task))
            }
        });
    }

    /// Will continually pull tasks from the queue until the queue has completed.
    fn thread_work<TaskFunc>(&self, per_task: &TaskFunc)
    where TaskFunc: Fn(Task) + Sync {
        while let Some(task) = self.receive_task() {
            (per_task)(task);
        }
    }

    /// Waits until there is a task available for this thread or returns None if the work
    /// queue has completed.
    /// # Note:
    /// This function also detects when the queue has completed. A queue is considered
    /// 'complete' if there are exactly as many threads waiting as there are threads
    /// in the pool. Adding/removing threads from the pool is not supported.
    fn receive_task(&self) -> Option<Task> {
        let mut data = self.data.lock();
        if !data.tasks.is_empty() {
            return data.tasks.pop();
        }
        loop {
            data.waiting_count += 1;
            // If all the threads in the queue are waiting, the queue is done.
            if data.waiting_count >= self.pool.current_num_threads() {
                data.has_shutdown = true;
                drop(data);
                self.cond.notify_all();
                return None;
            }

            self.cond.wait(&mut data);
            data.waiting_count -= 1;
            if data.has_shutdown {
                return None;
            } else if !data.tasks.is_empty() {
                return data.tasks.pop();
            }
        }
    }
}

struct WorkQueueData<T> {
    tasks: Vec<T>,
    waiting_count: usize,
    has_shutdown: bool,
}
impl<T> Default for WorkQueueData<T> {
    fn default() -> Self {
        WorkQueueData {
            tasks: Vec::new(),
            waiting_count: 0,
            has_shutdown: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use rayon::ThreadPoolBuilder;

    use super::*;
    use crate::sync::{
        AtomicUsize,
        Ordering,
    };

    #[test]
    fn single_thread_works() {
        let pool = Arc::new(ThreadPoolBuilder::new().num_threads(1).build().unwrap());
        let mut accum = AtomicUsize::new(0);
        let mut queue = WorkQueue::new(&pool);
        queue.add_task_mut(0usize);
        queue.add_task_mut(1usize);
        queue.add_task_mut(2usize);
        queue.work(&|x| {
            if x < 100 {
                queue.add_task(x + 1);
                accum.fetch_add(1, Ordering::SeqCst);
            }
        });
        assert_eq!(*accum.get_mut(), 100 + 99 + 98);
    }

    #[test]
    fn empty_queue_completes() {
        let pool = Arc::new(ThreadPoolBuilder::new().num_threads(1).build().unwrap());
        let queue = WorkQueue::new(&pool);
        queue.work(&|_: ()| ());
    }
}
