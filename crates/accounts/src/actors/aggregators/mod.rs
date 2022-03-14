pub mod accounts_state_aggregator;

use super::{Actor, CommandEnvelope};
use crate::broadcast::Broadcast;
use flume::Sender;

pub trait Aggregator {
    type Event;

    fn handle(&mut self, event: Self::Event);
}

type Envelope<TState> = CommandEnvelope<AggregatorRequests<TState>, AggregatorResponses>;

#[derive(Clone)]
pub struct AggregatorClient<TState>(Sender<Envelope<TState>>)
where
    TState: Clone + std::fmt::Debug;

impl<TState: Clone + std::fmt::Debug> AggregatorClient<TState> {
    pub async fn send_async(
        &self,
        payload: AggregatorRequests<TState>,
    ) -> Result<AggregatorResponses, ()> {
        let (callback, receiver) = flume::bounded(1);
        let envelope = Envelope { payload, callback };
        let _ = self.0.send_async(envelope).await;
        let response = receiver.recv_async().await.map_err(|_| ());

        response
    }
}

pub enum AggregatorRequests<TState> {
    Call(Box<dyn Fn(&TState) + Send>),
}

impl<TState> std::fmt::Debug for AggregatorRequests<TState> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Call(_) => f.debug_tuple("Call").finish(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum AggregatorResponses {
    Finished,
}

pub struct AggregatorActor<TState, TEvent> {
    state: TState,
    broadcast: Broadcast<TEvent>,
}

impl<TState, TEvent> std::fmt::Debug for AggregatorActor<TState, TEvent>
where
    TState: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AggregatorActor")
            .field("state", &self.state)
            .field("broadcast", &"...")
            .finish()
    }
}

#[async_trait::async_trait]
impl<TState, TEvent> Actor<AggregatorRequests<TState>, AggregatorResponses>
    for AggregatorActor<TState, TEvent>
where
    TState: 'static + Send + Clone + Aggregator<Event = TEvent> + std::fmt::Debug,
    TEvent: 'static + Send + Clone + std::fmt::Debug,
{
    type Client = AggregatorClient<TState>;

    fn new_client(
        &mut self,
        sender: flume::Sender<CommandEnvelope<AggregatorRequests<TState>, AggregatorResponses>>,
    ) -> Self::Client {
        AggregatorClient(sender)
    }

    async fn handle_request(
        &mut self,
        request: AggregatorRequests<TState>,
        callback: Sender<AggregatorResponses>,
    ) {
        let response = match request {
            AggregatorRequests::Call(f) => {
                let state = &self.state;
                f(state);
                AggregatorResponses::Finished
            }
        };
        let _ = callback.send_async(response).await;
    }

    async fn handle(
        mut self,
        sender: flume::Sender<CommandEnvelope<AggregatorRequests<TState>, AggregatorResponses>>,
        receiver: flume::Receiver<CommandEnvelope<AggregatorRequests<TState>, AggregatorResponses>>,
    ) {
        loop {
            tokio::select! {
                // handle requests
                command = receiver.recv_async() => {
                    match command {
                        Ok(CommandEnvelope { payload, callback }) => {
                            self.handle_request(payload, callback).await;
                        }
                        Err(err) => {
                            tracing::error!("{}", err);
                            break;
                        }
                    }
                },
                // aggregate events
                event = self.broadcast.recv_async() => {
                    self.state.handle(event.unwrap());
                },
                else => break,
            };
        }
        drop(sender);
    }
}

impl<TState: Aggregator + Default, TEvent> AggregatorActor<TState, TEvent> {
    pub fn new(broadcast: Broadcast<TEvent>) -> Self {
        Self {
            state: TState::default(),
            broadcast,
        }
    }
}
