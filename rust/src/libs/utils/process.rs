use std::cell::RefCell;
use std::collections::HashMap;
use std::any::Any;

thread_local! {
    static PROCESS_STORE: RefCell<HashMap<String, Box<dyn Any>>> = RefCell::new(HashMap::new());
}

pub struct Process;

impl Process {
    /// Stores a value under a key
    pub fn put<T: 'static>(key: &str, value: T) {
        PROCESS_STORE.with(|store| {
            store.borrow_mut().insert(key.to_string(), Box::new(value));
        });
    }

    /// Retrieves a value of type T from a key
    pub fn get<T: 'static + Clone>(key: &str) -> Option<T> {
        PROCESS_STORE.with(|store| {
            store.borrow()
                .get(key)
                .and_then(|v| v.downcast_ref::<T>().cloned())
        })
    }

    /// Removes a key-value pair
    pub fn delete(key: &str) {
        PROCESS_STORE.with(|store| {
            store.borrow_mut().remove(key);
        });
    }
}