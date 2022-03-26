use rust_decimal::{
    prelude::{FromPrimitive, Zero},
    Decimal,
};

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Currency {
    Bitcoin,
    Other { code: u64 },
}

impl std::ops::Mul<Currency> for u64 {
    type Output = Money;

    fn mul(self, rhs: Currency) -> Self::Output {
        Money {
            amount: Decimal::from_u64(self).unwrap(), //TODO remove this unwrap. When this fail?
            currency: rhs,
        }
    }
}

impl std::ops::Mul<Currency> for f64 {
    type Output = Money;

    fn mul(self, rhs: Currency) -> Self::Output {
        Money {
            amount: Decimal::from_f64(self).unwrap(), //TODO remove this unwrap. When this fail?
            currency: rhs,
        }
    }
}

impl std::ops::Mul<Currency> for Decimal {
    type Output = Money;

    fn mul(self, rhs: Currency) -> Self::Output {
        Money {
            amount: self,
            currency: rhs,
        }
    }
}

impl Currency {
    pub fn zero(self) -> Money {
        Money {
            amount: Decimal::ZERO,
            currency: self,
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct Money {
    pub amount: Decimal,
    pub currency: Currency,
}

impl From<Money> for rust_decimal::Decimal {
    fn from(money: Money) -> Self {
        money.amount
    }
}

#[derive(Clone, Debug)]
pub enum MoneyErrors {
    MismatchedCurrencies,
    Overflow,
    Underflow,
}

//TODO unfortunately a lot of Money fn cannot be const because Decimal fns are not const. :(
impl Money {
    pub fn new(currency: Currency) -> Self {
        Self {
            amount: Decimal::zero(),
            currency,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.amount.is_zero()
    }

    pub fn is_positive(&self) -> bool {
        self.amount.is_sign_positive()
    }

    pub fn is_negative(&self) -> bool {
        self.amount.is_sign_negative()
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, MoneyErrors> {
        if self.currency != rhs.currency {
            return Err(MoneyErrors::MismatchedCurrencies);
        }

        self.amount
            .checked_add(rhs.amount)
            .map(|amount| Money {
                amount,
                currency: self.currency,
            })
            .ok_or(MoneyErrors::Overflow)
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self, MoneyErrors> {
        if self.currency != rhs.currency {
            return Err(MoneyErrors::MismatchedCurrencies);
        }

        self.amount
            .checked_sub(rhs.amount)
            .map(|amount| Money {
                amount,
                currency: self.currency,
            })
            .ok_or(MoneyErrors::Underflow)
    }

    pub fn as_decimal(&self) -> Decimal {
        self.amount
    }
}

impl std::cmp::PartialEq<u64> for Money {
    fn eq(&self, other: &u64) -> bool {
        self.amount == Decimal::from_u64(*other).unwrap() //TODO remove unwrap
    }
}

impl std::ops::SubAssign<Decimal> for Money {
    fn sub_assign(&mut self, rhs: Decimal) {
        self.amount -= rhs;
    }
}

impl std::ops::SubAssign<&Decimal> for Money {
    fn sub_assign(&mut self, rhs: &Decimal) {
        self.amount -= rhs;
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    pub fn new_amount_is_zero() {
        let m = Money::new(Currency::Bitcoin);
        assert_eq!(m.amount, dec!(0));
    }

    // #[test]
    // pub fn cannot_operate_different_currencies() {
    //     todo!();
    // }

    // #[test]
    // pub fn sub_cannot_underflow() {
    //     todo!();
    // }
}
