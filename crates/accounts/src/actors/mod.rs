pub mod account;
pub mod account_manager;
pub mod account_shard;
pub mod aggregators;

pub struct CommandEnvelope<TRequest: std::fmt::Debug, TResponse> {
    payload: TRequest,
    callback: flume::Sender<TResponse>,
}

impl<TRequest: std::fmt::Debug, TResponse> std::fmt::Debug
    for CommandEnvelope<TRequest, TResponse>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandEnvelope")
            .field("payload", &self.payload)
            .field("callback", &"...")
            .finish()
    }
}

#[macro_export]
macro_rules! gen_client_extension_methods {
    (impl $trait_name:tt for $client_name:tt {
        $(fn $fn_name:tt ( $arg_name:tt: $arg_ty:ty ) -> $return_ty:ty; )*
    }) => {
        paste::paste! {
            #[derive(Debug)]
            pub enum [<$trait_name:camel Requests>] {
                $([<$fn_name:camel Request>] ($arg_ty),)*
            }

            unsafe impl Send for [<$trait_name:camel Requests>] {}

            $(
                impl From<$arg_ty> for [<$trait_name:camel Requests>] {
                    fn from(item: $arg_ty) -> Self {
                        [<$trait_name:camel Requests>]::[<$fn_name:camel Request>](item)
                    }
                }
            )*


            #[derive(Debug)]
            pub enum [<$trait_name:camel Responses>] {
                $([<$fn_name:camel Response>] ($return_ty),)*
            }

            unsafe impl Send for [<$trait_name:camel Responses>] {}

            $(
                impl From<$return_ty> for [<$trait_name:camel Responses>] {
                    fn from(item: $return_ty) -> Self {
                        [<$trait_name:camel Responses>]::[<$fn_name:camel Response>](item)
                    }
                }
            )*

            pub type Envelope = CommandEnvelope<[<$trait_name:camel Requests>], [<$trait_name:camel Responses>]>;

            impl $client_name {
                #[tracing::instrument(skip(self))]
                pub async fn send_async(&self, payload: [<$trait_name:camel Requests>]) -> Result<[<$trait_name:camel Responses>], ()> {
                    let (callback, receiver) = flume::bounded(1);
                    let envelope = Envelope {
                        payload,
                        callback
                    };
                    let _ = self.0.send_async(envelope).await;
                    let response = receiver.recv_async().await.map_err(|_| ());

                    response
                }

                $(
                    #[tracing::instrument(skip(self))]
                    pub fn [<send_ $fn_name:snake _async>](&self, payload: impl Into<$arg_ty> + std::fmt::Debug) -> impl std::future::Future<Output = Result<$return_ty, ()>> {
                        async fn [<send_ $fn_name:snake _impl>](s: $client_name, payload: impl Into<$arg_ty> + std::fmt::Debug) -> Result<$return_ty, ()> {
                            let payload = payload.into();

                            let (callback, receiver) = flume::bounded(1);
                            let envelope = Envelope {
                                payload: [<$trait_name:camel Requests>]::[<$fn_name:camel Request>](payload),
                                callback
                            };

                            let _ = s.0.send_async(envelope).await;
                            let response = receiver.recv_async().await;
                            match response {
                                Ok([<$trait_name:camel Responses>]::[<$fn_name:camel Response>](x)) => Ok(x),
                                _ => Err(())
                            }
                        }
                        [<send_ $fn_name:snake _impl>](self.clone(), payload)
                    }
                )*
            }
        }
    };
}

#[cfg(test)]
fn init_log() {
    let log_enabled = std::env::args().any(|x| x == "log=true");
    if log_enabled {
        use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;
        tracing_subscriber::Registry::default()
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .with(
                tracing_tree::HierarchicalLayer::new(2)
                    .with_targets(true)
                    .with_bracketed_fields(true),
            )
            .init();
    }
}

trait Spawn {
    type Output;

    fn spawn(self) -> Self::Output;
}

impl<TOutput, TFuture> Spawn for TFuture
where
    TOutput: 'static + Sized + Send,
    TFuture: 'static + std::future::Future<Output = TOutput>,
    Self: Send,
{
    type Output = tokio::task::JoinHandle<TOutput>;

    fn spawn(self) -> Self::Output {
        tokio::task::spawn(async move { self.await })
    }
}

#[async_trait::async_trait]
pub trait Actor<TRequest, TResponse>
where
    Self: 'static + Send + Sized,
    TRequest: 'static + Send + std::fmt::Debug,
    TResponse: 'static + Send + std::fmt::Debug,
{
    type Client;

    fn new_client(
        &mut self,
        sender: flume::Sender<CommandEnvelope<TRequest, TResponse>>,
    ) -> Self::Client;

    fn spawn(mut self) -> Self::Client {
        let (sender, receiver) = flume::unbounded();

        let client = self.new_client(sender.clone());
        tokio::task::spawn(self.handle(sender, receiver));
        client
    }

    fn set_sender(&mut self, _: flume::Sender<CommandEnvelope<TRequest, TResponse>>) {}

    async fn handle(
        mut self,
        sender: flume::Sender<CommandEnvelope<TRequest, TResponse>>,
        receiver: flume::Receiver<CommandEnvelope<TRequest, TResponse>>,
    ) {
        self.set_sender(sender.clone());

        loop {
            let msg = receiver.recv_async().await;
            match msg {
                Ok(CommandEnvelope { payload, callback }) => {
                    self.handle_request(payload, callback).await;
                }
                Err(err) => {
                    tracing::error!("{}", err);
                    break;
                }
            }
        }
        drop(sender);
    }

    async fn handle_request(&mut self, request: TRequest, callback: flume::Sender<TResponse>);
}
