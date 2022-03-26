use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;

use crate::domain::money::{Currency, Money, MoneyErrors};

use super::{events::AllEvents, DomainResult};

#[derive(Clone, Debug)]
pub struct Account {
    id: u32,
    amount: Money,
    ammounts: BTreeMap<u32, Decimal>,
    in_dispute: BTreeSet<u32>,
}

#[derive(Clone, Debug)]
pub enum AccountErrors {
    MoneyErrors(MoneyErrors),
    NegativeAmount,
    TransactionNotFound,
}

type AccountDomainResult<T> = DomainResult<T, AccountErrors, AllEvents>;

impl Account {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            amount: Currency::Bitcoin.zero(),
            ammounts: BTreeMap::new(),
            in_dispute: BTreeSet::new(),
        }
    }

    pub fn deposit(&mut self, transaction_id: u32, amount: Money) -> AccountDomainResult<()> {
        let mut events = vec![];

        match self.amount.checked_add(amount) {
            Ok(amount) => {
                self.ammounts.insert(transaction_id, amount.as_decimal());
                self.amount = amount;
                self.raise_account_updated(&mut events, transaction_id);
                AccountDomainResult::Ok { data: (), events }
            }
            Err(err) => AccountDomainResult::Err(AccountErrors::MoneyErrors(err)),
        }
    }

    pub fn withdraw(&mut self, transaction_id: u32, amount: Money) -> AccountDomainResult<()> {
        let mut events = vec![];

        match self.amount.checked_sub(amount) {
            Ok(amount) => {
                if amount.is_negative() {
                    AccountDomainResult::Err(AccountErrors::NegativeAmount)
                } else {
                    self.ammounts
                        .insert(transaction_id, amount.as_decimal() * Decimal::NEGATIVE_ONE);
                    self.amount = amount;
                    self.raise_account_updated(&mut events, transaction_id);
                    AccountDomainResult::Ok { data: (), events }
                }
            }
            Err(err) => AccountDomainResult::Err(AccountErrors::MoneyErrors(err)),
        }
    }

    pub fn dispute(&mut self, transaction_id: u32) -> AccountDomainResult<()> {
        let mut events = vec![];

        match self.ammounts.get(&transaction_id) {
            Some(amount) => match self.amount.checked_sub(*amount * self.amount.currency) {
                Ok(amount) => {
                    self.amount = amount;
                    self.in_dispute.insert(transaction_id);
                    self.raise_account_updated(&mut events, transaction_id);
                    AccountDomainResult::Ok { data: (), events }
                }
                Err(err) => AccountDomainResult::Err(AccountErrors::MoneyErrors(err)),
            },
            None => AccountDomainResult::Err(AccountErrors::TransactionNotFound),
        }
    }

    pub fn resolve(&mut self, transaction_id: u32) -> AccountDomainResult<()> {
        let mut events = vec![];

        if !self.in_dispute.remove(&transaction_id) {
            AccountDomainResult::Err(AccountErrors::TransactionNotFound)
        } else {
            match self.ammounts.get(&transaction_id) {
                Some(amount) => match self.amount.checked_add(*amount * self.amount.currency) {
                    Ok(amount) => {
                        self.amount = amount;
                        self.raise_account_updated(&mut events, transaction_id);
                        AccountDomainResult::Ok { data: (), events }
                    }
                    Err(err) => AccountDomainResult::Err(AccountErrors::MoneyErrors(err)),
                },
                None => AccountDomainResult::Err(AccountErrors::TransactionNotFound),
            }
        }
    }

    fn raise_account_updated(&self, events: &mut Vec<AllEvents>, transaction_id: u32) {
        let held = self
            .in_dispute
            .iter()
            .filter_map(|x| self.ammounts.get(x))
            .fold(Decimal::ZERO, |l, r| l + *r);

        events.push(AllEvents::AccountUpdated {
            account_id: self.id,
            transaction_id,
            amount: self.amount.into(),
            held,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::money::Currency::Bitcoin;
    use crate::quicktest_utils::*;
    use quickcheck_macros::*;

    #[test]
    pub fn ok_deposit() {
        let mut account = Account::new(0);
        account.deposit(0, 1 * Bitcoin).unwrap();
        assert!(account.amount == 1);
    }

    #[quickcheck]
    fn doundo_deposit_withdraw(amount: u64) -> bool {
        let mut account = Account::new(0);
        account.deposit(0, amount * Bitcoin).unwrap();
        account.withdraw(1, amount * Bitcoin).unwrap();

        account.amount.is_zero()
    }

    #[quickcheck]
    fn ok_deposit_bigger_than_withdraw(values: BigSmall<u64>) -> bool {
        let mut account = Account::new(0);
        account.deposit(0, values.big * Bitcoin).unwrap();
        account.withdraw(1, values.small * Bitcoin).unwrap();

        account.amount.is_positive()
    }

    #[quickcheck]
    fn err_withdraw_bigger_than_deposit(values: BigSmall<u64>) {
        let mut account = Account::new(0);
        account.deposit(0, values.small * Bitcoin).unwrap();
        account
            .withdraw(1, values.big * Bitcoin)
            .expect_err("Cannot withdraw more than was deposit (if only! :P)");
    }

    #[test]
    fn ok_deposit_dispute_resolve() {
        let mut account = Account::new(0);

        account.deposit(0, 1 * Bitcoin).unwrap();
        assert!(account.amount.as_decimal() == Decimal::ONE);

        account.dispute(0);
        assert!(account.amount.is_zero());

        account.resolve(0);
        assert!(account.amount.as_decimal() == Decimal::ONE);
    }

    //TODO test events
}
