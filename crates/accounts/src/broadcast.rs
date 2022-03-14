use tokio::task::JoinHandle;

pub struct Broadcast<T> {
    sender: tokio::sync::broadcast::Sender<T>,
    receiver: tokio::sync::broadcast::Receiver<T>,
}

impl<T: Clone> Default for Broadcast<T> {
    fn default() -> Self {
        let (sender, receiver) = tokio::sync::broadcast::channel(1024); //TODO magic number
        Self { sender, receiver }
    }
}

impl<T> Clone for Broadcast<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.sender.subscribe(),
        }
    }
}

#[derive(Clone)]
pub struct Sender<T> {
    sender: tokio::sync::broadcast::Sender<T>,
}

pub struct BroadcastRecorder<T> {
    cancel_sender: flume::Sender<()>,
    handle: JoinHandle<Vec<T>>,
}

impl<T> BroadcastRecorder<T> {
    pub async fn stop(self) -> Vec<T> {
        let _ = self.cancel_sender.send_async(()).await;
        self.handle.await.unwrap()
    }
}

impl<T: Clone> Broadcast<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn broadcast_all(&self, events: impl Iterator<Item = T>) {
        for event in events {
            let _ = self.sender.send(event);
        }
    }

    pub async fn recv_async(&mut self) -> Result<T, tokio::sync::broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    pub fn to_sender(self) -> tokio::sync::broadcast::Sender<T> {
        self.sender
    }

    pub fn to_receiver(self) -> tokio::sync::broadcast::Receiver<T> {
        self.receiver
    }

    pub fn spawn_recorder(self) -> BroadcastRecorder<T>
    where
        T: 'static + Send,
    {
        let Broadcast { mut receiver, .. } = self;
        let (cancel_sender, cancel_receiver) = flume::bounded(1);

        let handle = tokio::task::spawn(async move {
            let mut events = vec![];
            loop {
                tokio::select! {
                    event = receiver.recv() => events.push(event.unwrap()),
                    _ = cancel_receiver.recv_async() => break,
                }
            }
            events
        });

        BroadcastRecorder {
            cancel_sender,
            handle,
        }
    }
}
