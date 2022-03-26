use std::collections::BTreeMap;

use flume::Sender;
use tracing::warn;

use crate::{
    broadcast::Broadcast,
    domain::{
        account::{Account, AccountErrors},
        events::AllEvents,
        money::Money,
        DomainResult,
    },
    gen_client_extension_methods,
};

use super::{Actor, CommandEnvelope};

#[derive(Clone, Debug)]
pub struct DepositRequest {
    pub account_id: u32,
    pub transaction_id: u32,
    pub amount: Money,
}

#[derive(Clone, Debug)]
pub enum DepositResponse {
    Ok,
    Error(AccountErrors),
}

#[derive(Clone, Debug)]
pub struct WithdrawRequest {
    pub account_id: u32,
    pub transaction_id: u32,
    pub amount: Money,
}

#[derive(Clone, Debug)]
pub enum WithdrawResponse {
    Ok,
    Error(AccountErrors),
}

#[derive(Clone, Debug)]
pub struct DisputeRequest {
    pub account_id: u32,
    pub transaction_id: u32,
}

#[derive(Clone, Debug)]
pub enum DisputeResponse {
    Ok,
    Error(AccountErrors),
}

#[derive(Clone, Debug)]
pub struct ResolveRequest {
    pub account_id: u32,
    pub transaction_id: u32,
}

#[derive(Clone, Debug)]
pub enum ResolveResponse {
    Ok,
    Error(AccountErrors),
}

#[derive(Clone, Debug)]
pub struct ChargebackRequest {
    pub account_id: u32,
    pub transaction_id: u32,
}

#[derive(Clone, Debug)]
pub enum ChargebackResponse {
    Ok,
    Error(AccountErrors),
}

#[derive(Clone, Copy, Debug)]
pub struct Accept;

#[derive(Clone)]
pub struct AccountClient(Sender<Envelope>);

gen_client_extension_methods! {
    impl Account for AccountClient {
        fn deposit(_: DepositRequest) -> DepositResponse;
        fn withdraw(_: WithdrawRequest) -> WithdrawResponse;
        fn dispute(_: DisputeRequest) -> DisputeResponse;
        fn resolve(_: ResolveRequest) -> ResolveResponse;
        fn chargeback(_: ChargebackRequest) -> ChargebackResponse;
        fn accept_request(_: Accept) -> Accept;
    }
}

impl AccountRequests {
    pub fn get_account_id(&self) -> u32 {
        match self {
            AccountRequests::DepositRequest(x) => x.account_id,
            AccountRequests::WithdrawRequest(x) => x.account_id,
            AccountRequests::DisputeRequest(x) => x.account_id,
            AccountRequests::ResolveRequest(x) => x.account_id,
            AccountRequests::ChargebackRequest(x) => x.account_id,
            AccountRequests::AcceptRequestRequest(_) => {
                panic!("This message does not have account_id.")
            }
        }
    }

    pub fn get_transaction_id(&self) -> u32 {
        match self {
            AccountRequests::DepositRequest(x) => x.transaction_id,
            AccountRequests::WithdrawRequest(x) => x.transaction_id,
            AccountRequests::DisputeRequest(x) => x.transaction_id,
            AccountRequests::ResolveRequest(x) => x.transaction_id,
            AccountRequests::ChargebackRequest(x) => x.transaction_id,
            AccountRequests::AcceptRequestRequest(_) => {
                panic!("This message does not have transaction_id.")
            }
        }
    }
}

enum Disputes {
    Dispute(DisputeRequest, Sender<AccountResponses>),
    Resolve(ResolveRequest, Sender<AccountResponses>),
    Chargeback(ChargebackRequest, Sender<AccountResponses>),
}

pub struct AccountActor {
    account: Account,
    requests: BTreeMap<u32, (AccountRequests, Sender<AccountResponses>)>,
    disputes: BTreeMap<u32, Vec<Disputes>>,
    broadcast: Broadcast<AllEvents>,
    sender: flume::Sender<CommandEnvelope<AccountRequests, AccountResponses>>,
}

impl std::fmt::Debug for AccountActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountActor")
            .field("account", &self.account)
            .field("broadcast", &"...")
            .finish()
    }
}

#[async_trait::async_trait]
impl Actor<AccountRequests, AccountResponses> for AccountActor {
    type Client = AccountClient;

    fn new_client(
        &mut self,
        sender: flume::Sender<CommandEnvelope<AccountRequests, AccountResponses>>,
    ) -> Self::Client {
        AccountClient(sender)
    }

    fn set_sender(
        &mut self,
        sender: flume::Sender<CommandEnvelope<AccountRequests, AccountResponses>>,
    ) {
        self.sender = sender;
    }

    #[tracing::instrument(skip(self))]
    async fn handle_request(
        &mut self,
        request: AccountRequests,
        callback: Sender<AccountResponses>,
    ) {
        use AccountRequests::*;
        match request {
            AcceptRequestRequest(_) => self.accept_request().await,

            DepositRequest(_) => self.schedule_request(request, callback),
            WithdrawRequest(_) => self.schedule_request(request, callback),

            DisputeRequest(r) => self.add_dispute(r.transaction_id, Disputes::Dispute(r, callback)),
            ResolveRequest(r) => self.add_dispute(r.transaction_id, Disputes::Resolve(r, callback)),
            ChargebackRequest(r) => {
                self.add_dispute(r.transaction_id, Disputes::Chargeback(r, callback))
            }
        }
    }
}

impl AccountActor {
    pub fn new(account: Account, broadcast: Broadcast<AllEvents>) -> Self {
        let (sender, _) = flume::unbounded();
        Self {
            account,
            broadcast,
            requests: BTreeMap::new(),
            sender,
            disputes: BTreeMap::new(),
        }
    }

    #[tracing::instrument(skip(self), ret)]
    pub fn handle_deposit(
        &mut self,
        transaction_id: u32,
        deposit: DepositRequest,
    ) -> DepositResponse {
        match self.account.deposit(transaction_id, deposit.amount) {
            DomainResult::Ok { mut events, .. } => {
                self.broadcast.broadcast_all(events.drain(..));
                DepositResponse::Ok
            }
            DomainResult::Err(err) => DepositResponse::Error(err),
        }
    }

    #[tracing::instrument(skip(self), ret)]
    pub fn handle_withdraw(
        &mut self,
        transaction_id: u32,
        withdraw: WithdrawRequest,
    ) -> WithdrawResponse {
        match self.account.withdraw(transaction_id, withdraw.amount) {
            DomainResult::Ok { mut events, .. } => {
                self.broadcast.broadcast_all(events.drain(..));
                WithdrawResponse::Ok
            }
            DomainResult::Err(err) => WithdrawResponse::Error(err),
        }
    }

    #[tracing::instrument(skip(self), ret)]
    pub fn handle_dispute(
        &mut self,
        transaction_id: u32,
        dispute: DisputeRequest,
    ) -> DisputeResponse {
        match self.account.dispute(transaction_id) {
            DomainResult::Ok { mut events, .. } => {
                self.broadcast.broadcast_all(events.drain(..));
                DisputeResponse::Ok
            }
            DomainResult::Err(err) => DisputeResponse::Error(err),
        }
    }

    #[tracing::instrument(skip(self), ret)]
    pub fn handle_resolve(
        &mut self,
        transaction_id: u32,
        resolve: ResolveRequest,
    ) -> ResolveResponse {
        match self.account.resolve(transaction_id) {
            DomainResult::Ok { mut events, .. } => {
                self.broadcast.broadcast_all(events.drain(..));
                ResolveResponse::Ok
            }
            DomainResult::Err(err) => ResolveResponse::Error(err),
        }
    }

    #[tracing::instrument(skip(self), ret)]
    pub fn handle_chargeback(
        &mut self,
        transaction_id: u32,
        chargeback: ChargebackRequest,
    ) -> ChargebackResponse {
        match self.account.chargeback(transaction_id) {
            DomainResult::Ok { mut events, .. } => {
                self.broadcast.broadcast_all(events.drain(..));
                ChargebackResponse::Ok
            }
            DomainResult::Err(err) => ChargebackResponse::Error(err),
        }
    }

    // To allow out of order delivery of accounts operations, when a request
    // arrives, we wait 100ms before accepting it.
    // I am not 100% sure of optimize is to one spawn per message here. Tokio
    // correctly implements Timing Wheel (https://github.com/tokio-rs/tokio/blob/master/tokio/src/time/driver/wheel/mod.rs),
    // SO my bet this is fine.
    #[tracing::instrument(skip(self))]
    pub fn schedule_request(
        &mut self,
        request: AccountRequests,
        callback: Sender<AccountResponses>,
    ) {
        let current_transaction_id = request.get_transaction_id();
        let _ = self
            .requests
            .insert(current_transaction_id, (request, callback));

        let sender = self.sender.clone();
        tokio::task::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await; //TODO magic number
            let (callback_sender, callback_recv) = flume::bounded(1);
            let _ = sender
                .send_async(CommandEnvelope {
                    payload: AccountRequests::AcceptRequestRequest(Accept),
                    callback: callback_sender,
                })
                .await;
            let _ = callback_recv.recv_async().await;
        });
    }

    // Now we pop the earlier request, order by transaction id,
    // and accept it.
    #[tracing::instrument(skip(self))]
    pub async fn accept_request(&mut self) {
        // Ideally ```self.requests.pop_first()```, but it is still unstable.
        let item = match self.requests.iter().next().map(|x| *x.0) {
            None => return,
            Some(key) => self.requests.remove(&key),
        };

        if let Some((request, callback)) = item {
            let transaction_id = request.get_transaction_id();
            use AccountRequests::*;
            let response: AccountResponses = match request {
                DepositRequest(r) => self.handle_deposit(transaction_id, r).into(),
                WithdrawRequest(r) => self.handle_withdraw(transaction_id, r).into(),
                _ => unreachable!("Should never postpone non account operations"),
            };

            let _ = callback.send_async(response).await;

            // Now run all known disputes into this transaction
            if let Some(disputes) = self.disputes.remove(&transaction_id) {
                for dispute in disputes {
                    match dispute {
                        Disputes::Dispute(r, callback) => {
                            let r = self.handle_dispute(transaction_id, r);
                            let _ = callback.send_async(r.into()).await;
                        }
                        Disputes::Resolve(r, callback) => {
                            let r = self.handle_resolve(transaction_id, r);
                            let _ = callback.send_async(r.into()).await;
                        }
                        Disputes::Chargeback(r, callback) => {
                            let r = self.handle_chargeback(transaction_id, r);
                            let _ = callback.send_async(r.into()).await;
                        }
                    }
                }
            }
        }
    }

    fn add_dispute(&mut self, transaction_id: u32, dispute: Disputes) {
        let disputes = self.disputes.entry(transaction_id).or_default();
        disputes.push(dispute);
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        actors::{
            account::{DepositResponse, WithdrawResponse},
            init_log, Actor, Spawn,
        },
        broadcast::Broadcast,
        domain::{account::Account, money::Currency::*},
    };

    use super::{AccountActor, DepositRequest, WithdrawRequest};

    #[tokio::test]
    pub async fn err_incorrectly_waiting_on_out_of_order() {
        init_log();

        let broadcast = Broadcast::new();
        let account = Account::new(0);
        let account = AccountActor::new(account, broadcast.clone()).spawn();

        // Because account accepts out of order delivery
        // we cannot .await this sends, otherwise we will be
        // waiting them to be accepted
        // This is why this part fails
        let response = account
            .send_withdraw_async(WithdrawRequest {
                account_id: 0,
                transaction_id: 1,
                amount: 0.5 * Bitcoin,
            })
            .await;
        assert!(matches!(response, Ok(WithdrawResponse::Error(_))));

        let response = account
            .send_deposit_async(DepositRequest {
                account_id: 0,
                transaction_id: 0,
                amount: 1 * Bitcoin,
            })
            .await;
        assert!(matches!(response, Ok(DepositResponse::Ok)));
    }

    #[tokio::test]
    pub async fn ok_correctly_sending_out_of_order() {
        init_log();

        let broadcast = Broadcast::new();
        let account = Account::new(0);
        let account = AccountActor::new(account, broadcast.clone()).spawn();

        // see [err_incorrectly_waiting_on_out_of_order]
        // Now we correctly send and deal with out of order.
        let response1 = account
            .send_withdraw_async(WithdrawRequest {
                account_id: 0,
                transaction_id: 1,
                amount: 0.5 * Bitcoin,
            })
            .spawn();

        let response2 = account
            .send_deposit_async(DepositRequest {
                account_id: 0,
                transaction_id: 0,
                amount: 1 * Bitcoin,
            })
            .await;

        assert!(matches!(response1.await, Ok(Ok(WithdrawResponse::Ok))));
        assert!(matches!(response2, Ok(DepositResponse::Ok)));
    }
}
