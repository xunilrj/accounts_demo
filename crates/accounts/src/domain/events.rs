use rust_decimal::Decimal;

#[derive(Clone, Debug)]
pub enum AllEvents {
    AccountUpdated {
        account_id: u32,
        transaction_id: u32,
        amount: Decimal,
        held: Decimal,
    },
}
