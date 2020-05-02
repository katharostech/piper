//! Contains the [`ChangeNotifier`] type that can be used to listen to changes to variable

use std::sync::Arc;

use crate::event::{Event, EventListener};

/// A wrapper type for subscribing to changes to the inner type
#[derive(Debug)]
pub struct ChangeNotifier<T> {
    inner: T,
    event: Arc<Event>,
}

impl<T: Clone> Clone for ChangeNotifier<T> {
    fn clone(&self) -> Self {
        ChangeNotifier {
            inner: self.inner.clone(),
            event: self.event.clone(),
        }
    }
}

impl<T> ChangeNotifier<T> {
    /// Create a new [`ChangeNotifier`] wrapping the given data.
    pub fn new(data: T) -> Self {
        ChangeNotifier {
            inner: data,
            event: Arc::new(Event::new()),
        }
    }

    /// Update the inner data by passing a closure to do the mutation
    ///
    /// > **Note:** Listeners will *only* be notified to changes of the inner data
    /// > if this function is called and they will be notified regardless of any real
    /// > change to the inner data if this function is called.
    pub fn update<R, F>(&mut self, apply_update: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        // Apply the update to the inner type
        let ret = apply_update(&mut self.inner);

        // Notify all listeners of the change
        self.event.notify_all();

        // Return the return value of the apply update function
        ret
    }

    /// Get an event listener for changes to the inner data
    pub fn listen(&self) -> EventListener {
        self.event.listen()
    }
}

// Dereference to the inner data
impl<T> std::ops::Deref for ChangeNotifier<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use smol::{Task, Timer};
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering::SeqCst;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn smoke() {
        smol::run(async move {
            // Create a change detected atomic bool
            let mut myvar = ChangeNotifier::new(AtomicBool::new(false));
            // Add a listener for myvar
            let myvar_listener = myvar.listen();

            // Create another change detector we can use to make sure the other task received
            // our update.
            let received_update = ChangeNotifier::new(Arc::new(AtomicBool::new(false)));

            // Add a listener for received_update
            let received_update_listener = received_update.listen();

            // Spawn a task that waits for myvar to change
            let mut received_update_ = received_update.clone();
            Task::spawn(async move {
                // Wait for the var to change
                myvar_listener.await;

                // State that we received the update
                received_update_.update(|x| x.store(true, SeqCst));
            })
            .detach();
            // Make sure the update has not yet been received
            assert!(received_update.load(SeqCst) == false);

            // Update the bool
            myvar.update(|x| x.store(true, SeqCst));

            // Wait for the child task to receive the message and tell us that it has received
            // the update to myvar.
            received_update_listener.await;

            // Make sure the update was received
            assert!(received_update.load(SeqCst) == true);
        });
    }
}
