use crate::domain::money::{Currency, Money, MoneyErrors};

use super::{events::AllEvents, DomainResult};

#[derive(Clone, Debug)]
pub struct Account {
    id: u32,
    amount: Money,
}

#[derive(Clone, Debug)]
pub enum AccountErrors {
    MoneyErrors(MoneyErrors),
    NegativeAmount,
}

type AccountDomainResult<T> = DomainResult<T, AccountErrors, AllEvents>;

impl Account {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            amount: Currency::Bitcoin.zero(),
        }
    }

    pub fn deposit(&mut self, transaction_id: u32, amount: Money) -> AccountDomainResult<()> {
        let mut events = vec![];

        match self.amount.checked_add(amount) {
            Ok(amount) => {
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
                    self.amount = amount;
                    self.raise_account_updated(&mut events, transaction_id);
                    AccountDomainResult::Ok { data: (), events }
                }
            }
            Err(err) => AccountDomainResult::Err(AccountErrors::MoneyErrors(err)),
        }
    }

    fn raise_account_updated(&self, events: &mut Vec<AllEvents>, transaction_id: u32) {
        events.push(AllEvents::AccountUpdated {
            account_id: self.id,
            transaction_id,
            amount: self.amount.into(),
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

    //TODO test events
}
