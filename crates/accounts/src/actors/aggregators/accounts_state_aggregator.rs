use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::domain::events::AllEvents;

use super::{Aggregator, AggregatorActor};

#[derive(Clone, Debug)]
pub struct AccountState {
    pub client: u32,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

#[derive(Default, Clone, Debug)]
pub struct AccountsStateAggregator {
    pub accounts: HashMap<u32, AccountState>,
}

impl Aggregator for AccountsStateAggregator {
    type Event = AllEvents;

    fn handle(&mut self, event: AllEvents) {
        match event {
            AllEvents::AccountUpdated {
                account_id,
                amount,
                held,
                locked,
                ..
            } => {
                let state = self
                    .accounts
                    .entry(account_id)
                    .or_insert_with(|| AccountState {
                        client: account_id,
                        available: Decimal::ZERO,
                        held: Decimal::ZERO,
                        total: Decimal::ZERO,
                        locked: false,
                    });

                state.locked = locked;
                state.available = amount;
                state.held = held;
                state.total = state.available + state.held;
            }
        }
    }
}

pub type AccountsStateActor = AggregatorActor<AccountsStateAggregator, AllEvents>;
