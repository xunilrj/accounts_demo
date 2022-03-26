use flume::Sender;
use hashring::HashRing;

use crate::gen_client_extension_methods;

use super::{
    account::{AccountRequests, AccountResponses},
    account_manager::AccountManagerClient,
    Actor, CommandEnvelope,
};

#[derive(Clone)]
pub struct AccountShardClient(Sender<Envelope>);

gen_client_extension_methods! {
    impl AccountShard for AccountShardClient {
        fn account(_: AccountRequests) -> AccountResponses;
    }
}

pub struct AccountShardActor {
    ring: HashRing<AccountManagerClient>,
}

#[async_trait::async_trait]
impl Actor<AccountShardRequests, AccountShardResponses> for AccountShardActor {
    type Client = AccountShardClient;

    fn new_client(
        &mut self,
        sender: flume::Sender<CommandEnvelope<AccountShardRequests, AccountShardResponses>>,
    ) -> Self::Client {
        AccountShardClient(sender)
    }

    async fn handle_request(
        &mut self,
        request: AccountShardRequests,
        callback: Sender<AccountShardResponses>,
    ) {
        use AccountShardRequests::*;
        match request {
            AccountRequest(r) => self.redirect_request(r, callback),
        };
    }
}

impl AccountShardActor {
    pub fn new(clients: Vec<AccountManagerClient>) -> Self {
        let mut ring = HashRing::new();
        for client in clients {
            ring.add(client);
        }

        Self { ring }
    }

    #[tracing::instrument(skip(self, callback))]
    pub fn redirect_request(
        &mut self,
        request: AccountRequests,
        callback: Sender<AccountShardResponses>,
    ) {
        let account_id = request.get_account_id();
        let account = self.ring.get(&account_id).cloned().unwrap(); //TODO remove unwrap

        tokio::task::spawn(async move {
            match account.send_account_async(request).await {
                Ok(response) => {
                    let _ = callback.send_async(response.into()).await;
                }
                Err(err) => {
                    tracing::warn!("{:?}", err);
                    let _ = callback.send_async(AccountShardResponses::Error(err)).await;
                }
            }
        });
    }
}
