use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::{Condvar, Mutex, RwLock};
use thiserror::Error;

use crate::sync::Notifier;

#[derive(Error, Debug)]
enum MyError {
    #[error("oups")]
    TopicDoesNotExist(String),
}

#[derive(Debug, Clone)]
pub struct Aqueduc {
    db: Arc<DashMap<TopicId, Arc<Topic>>>,
}

type TopicId = &'static str;

#[derive(Debug)]
pub struct Topic {
    notifier: Notifier,
    vec: RwLock<Vec<Droplet>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Droplet {
    Data,
}

impl Aqueduc {
    pub fn new() -> Aqueduc {
        Aqueduc {
            db: Arc::new(DashMap::new()),
        }
    }

    /// Create a new topic
    pub fn new_topic(&self, topic_id: TopicId) {
        println!("> new topic: {}", topic_id);

        if !self.db.contains_key(topic_id) {
            self.db.insert(topic_id, Arc::new(Topic::new()));
        }
    }

    /// Insert a droplet at the end of a topic
    pub fn put(&self, topic_id: TopicId, droplet: Droplet) {
        println!("> topic {}: put droplet", topic_id);

        match self.db.get(topic_id) {
            Some(t) => {
                let mut guard = t.vec.write();

                guard.push(droplet);

                t.notifier.notify();
            }
            None => (),
        }
    }

    /// Get a droplet from a topic
    pub fn get(&self, topic_id: TopicId, index: usize) -> Option<Droplet> {
        println!("> topic {}: get droplet {}", topic_id, index);

        match self.db.get(topic_id) {
            Some(t) => {
                let guard = t.vec.read();

                guard.get(index).cloned()
            }
            None => None,
        }
    }

    /// Wait for a new droplet on a topic
    /// This function blocks until a new droplet is available
    /// Note: Several elements can be added to the same topic while waiting
    pub fn wait_for_notification(&self, topic_id: TopicId) {
        println!("> topic {}: wait", topic_id);

        match self.db.get(topic_id) {
            Some(t) => {
                let n = t.notifier.wait();
            }
            None => (),
        }
    }

    /// Get the latest index of a topic.
    /// Returns 0 if the topic is empty.
    fn latest_idx(&self, topic_id: TopicId) -> usize {
        0
    }

    /// Open a topic and return an error if it does not exist
    fn open_topic(&self, topic_id: TopicId) -> Result<Arc<Topic>, MyError> {
        println!("Opening topic {}", topic_id);

        match self.db.get(topic_id) {
            Some(r) => Ok(r.clone()),
            None => Err(MyError::TopicDoesNotExist(topic_id.to_string())),
        }
    }

    /// Wait for new droplets on a topic and call
    /// the given function for each new addition.
    /// This function blocks until a new droplet is available
    /// and stops when the given function returns false.
    pub fn listen<F>(&self, topic_id: TopicId, f: F)
    where
        F: Fn(Droplet) -> bool,
    {
    }

    /// Wait for new droplets on a topic and call
    /// the given function for each new addition.
    /// This function blocks until a new droplet is available
    /// and stops when the given function returns false.
    pub fn listen_mut<F>(&self, topic_id: TopicId, f: F)
    where
        F: FnMut(Droplet) -> bool,
    {
    }

    /// Wait for new droplets on a topic and call
    /// the given function for each new addition.
    /// This function blocks until a new droplet is available
    /// and stops when the given function returns false.
    /// The callback will be called for each droplet after the
    /// given index. If the index is higher than the current topic size,
    /// then the callback will wait until the topic reach the proper size.
    pub fn listen_after<F>(&self, topic_id: TopicId, x: usize, f: F)
    where
        F: Fn(Droplet) -> bool,
    {
    }

    /// Spawn a new thread that will call the given function reapeatingly
    /// and publish the resulting droplet on the given topic.
    /// The loop stops when the given function returns None.
    pub fn spawn<F>(&self, topic_id: TopicId, f: F)
    where
        F: Fn() -> Option<Droplet>,
    {
    }

    /// Spawn a new thread that will call the given function reapeatingly
    /// and publish the resulting droplet on the given topic.
    /// The loop stops when the given function returns None.
    pub fn spawn_mut<F>(&self, topic_id: TopicId, f: F)
    where
        F: FnMut() -> Option<Droplet>,
    {
    }

    pub fn wait2<F>(&self, topic_id: TopicId, mut f: F)
    where
        F: FnMut(Droplet) -> bool,
    {
        println!("> topic {}: wait2", topic_id);

        let topic = self.open_topic(topic_id).unwrap();

        let mut x = {
            let guard = topic.vec.read();
            if guard.len() == 0 {
                0
            } else {
                guard.len() - 1
            }
        };

        'outer: loop {
            topic.notifier.wait();

            let guard = topic.vec.read();
            while x < guard.len() {
                if !f(guard.get(x).unwrap().clone()) {
                    break 'outer;
                }
                x += 1;
            }
        }
    }
}

impl Topic {
    pub fn new() -> Topic {
        Topic {
            notifier: Notifier::new(),
            vec: RwLock::new(Vec::new()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::{thread, time::Duration};

    #[test]
    fn test_actor6_0() {
        let aq = Aqueduc::new();
        let bq = aq.clone();

        aq.new_topic("hello");

        // TODO:
        // spawn with function that reads from a topic and then publish on another ?
        // topicID = Enum ? with a wildcard selector ?
        thread::spawn(move || {
            bq.put("hello", Droplet::Data);
            bq.put("hello", Droplet::Data);
        });

        aq.wait_for_notification("hello");

        assert_eq!(aq.get("hello", 0).unwrap(), Droplet::Data);
        assert_eq!(aq.get("hello", 1).unwrap(), Droplet::Data);
    }

    #[test]
    fn test_actor6_1() {
        let aq = Aqueduc::new();
        let bq = aq.clone();

        aq.new_topic("hello");

        thread::spawn(move || {
            let mut i = 0;

            while i < 10 {
                bq.put("hello", Droplet::Data);

                i += 1;
            }

            thread::sleep(Duration::from_millis(100));

            bq.new_topic("ello2");
            bq.new_topic("ello3");

            bq.put("ello2", Droplet::Data);
            bq.put("ello2", Droplet::Data);
            bq.put("ello2", Droplet::Data);
        });

        let mut j = 0;
        aq.wait2("hello", |droplet| {
            println!("> droplet {}", j);

            assert_eq!(droplet, Droplet::Data);

            j += 1;

            j < 10
        });
    }

    #[test]
    fn test_actor6_2() {
        let aq = Aqueduc::new();

        // TODO: new api required
        // topic() -> Arc<Topic> ?
        // aq.topic("hello")
        //     .iter()
        //     .map(|droplet| println!("{}", droplet));
    }
}
