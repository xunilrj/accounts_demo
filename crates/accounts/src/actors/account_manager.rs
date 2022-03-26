use super::Actor;
use super::{
    account::{AccountClient, AccountResponses},
    CommandEnvelope,
};
use crate::broadcast::Broadcast;
use crate::domain::account::Account;
use crate::domain::events::AllEvents;
use crate::{
    actors::account::{AccountActor, AccountRequests},
    gen_client_extension_methods,
};
use flume::Sender;
use std::collections::HashMap;

#[derive(Clone)]
pub struct AccountManagerClient(Sender<Envelope>, u64);

impl std::hash::Hash for AccountManagerClient {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}

gen_client_extension_methods! {
    impl AccountManager for AccountManagerClient {
        fn account(_: AccountRequests) -> AccountResponses;
    }
}

pub struct AccountManagerActor {
    id: u64,
    accounts: HashMap<u32, AccountClient>,
    broadcast: Broadcast<AllEvents>,
}

impl std::fmt::Debug for AccountManagerActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountManagerActor")
            .field("id", &self.id)
            .field("accounts", &"[..]")
            .field("broadcast", &"...")
            .finish()
    }
}

#[async_trait::async_trait]
impl Actor<AccountManagerRequests, AccountManagerResponses> for AccountManagerActor {
    type Client = AccountManagerClient;

    fn new_client(
        &mut self,
        sender: flume::Sender<CommandEnvelope<AccountManagerRequests, AccountManagerResponses>>,
    ) -> Self::Client {
        AccountManagerClient(sender, self.id)
    }

    async fn handle_request(
        &mut self,
        request: AccountManagerRequests,
        callback: Sender<AccountManagerResponses>,
    ) {
        match request {
            AccountManagerRequests::AccountRequest(request) => {
                self.handle_account_request(request, callback)
            }
        }
    }
}

impl AccountManagerActor {
    pub fn new(id: u64, broadcast: Broadcast<AllEvents>) -> Self {
        Self {
            id,
            accounts: HashMap::new(),
            broadcast,
        }
    }

    #[tracing::instrument(skip(broadcast))]
    fn new_actor(id: u32, broadcast: Broadcast<AllEvents>) -> AccountClient {
        let account = Account::new(id);
        AccountActor::new(account, broadcast).spawn()
    }

    #[tracing::instrument(skip(self, callback))]
    pub fn handle_account_request(
        &mut self,
        request: AccountRequests,
        callback: Sender<AccountManagerResponses>,
    ) {
        let account_id = request.get_account_id();
        let account = self
            .accounts
            .entry(account_id)
            .or_insert_with(|| Self::new_actor(account_id, self.broadcast.clone()))
            .clone();

        tokio::task::spawn(async move {
            match account.send_async(request.clone()).await {
                Ok(response) => {
                    let _ = callback.send_async(response.into()).await;
                }
                Err(err) => {
                    tracing::warn!("{:?} {:?}", request, err);
                    let _ = callback.send_async(AccountManagerResponses::Error(err));
                }
            }
        });
    }
}
