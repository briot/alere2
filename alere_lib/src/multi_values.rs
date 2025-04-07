use crate::commodities::{Commodity, CommodityId};
use crate::formatters::Formatter;
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Operation {
    // The amount of the transaction, as seen on the bank statement.
    // This could be a number of shares when the account is a Stock account, for
    // instance, or a number of EUR for a checking account.
    //
    // For instance:
    // * a 1000 EUR transaction for an account in EUR. In this case, value is
    //   useless and does not provide any additional information.
    //       operation = Credit(1000 EUR)
    Credit(MultiValue),

    // Buying shares
    //
    // For instance:
    // * an ATM operation of 100 USD for the same account in EUR while abroad.
    //   Exchange rate at the time: 0.85 EUR = 1 USD.  Also assume there is a
    //   bank fee that applies.
    //      split1: account=checking account
    //              operation=BuyAmount(-100USD, -85EUR)
    //      split2: account=expense:cash  operation=Credit(84.7 EUR)
    //      split3: account=expense:fees  operation=Credit(0.3 EUR)
    //   So value is used to show you exactly the amount you manipulated. The
    //   exchange rate can be computed from qty and value.
    //
    // * Buying 10 shares for AAPL at 120 USD. There are several splits here,
    //   one where we increase the number of shares in the STOCK account.
    //   The money came from an investment account in EUR, which has its own
    //   split for the same transaction:
    //       split1: account=stock       BuyAmount(10 APPL, amount=1200 USD)
    //       split2: account=investment  BuyAmount(-1020USD, amount=-1200 USD)
    BuyAmount {
        qty: Value,
        amount: Value,
    },
    BuyPrice {
        qty: Value,
        price: Value,
    },
    AddShares {
        qty: Value,
    },

    // Reinvest dividends and buy shares.
    Reinvest {
        shares: MultiValue,
        amount: MultiValue,
    },

    // There were some dividends for one of the stocks   The amount will be
    // visible in other splits.
    // This only registers there was some dividend, but the amount will be
    // found in other splits associated with the same transaction
    Dividend,

    // Used for stock splits.  The number of shares is multiplied by the ratio,
    // and their value divided by the same ratio.
    Split {
        ratio: Decimal,
        commodity: Commodity,
    },
}

/// A value includes zero or more pairs (amount, commodity) to express prices
/// or account balances.
/// It can represent values like   1 EUR + 3 USD

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MultiValue(InnerValue);

/// A value is for a single commodity

#[derive(Clone, Debug, PartialEq)]
pub struct Value {
    pub amount: Decimal,
    pub commodity: Commodity,
}

impl Value {
    pub fn zero(commodity: &Commodity) -> Self {
        Value {
            commodity: commodity.clone(),
            amount: Decimal::ZERO,
        }
    }

    pub fn abs(&self) -> Value {
        Value {
            amount: self.amount.abs(),
            commodity: self.commodity.clone(),
        }
    }

    pub fn display(&self, format: &Formatter) -> String {
        format.display_from_commodity(self.amount, &self.commodity)
    }
}

impl core::ops::Div<Decimal> for &Value {
    type Output = Value;

    fn div(self, rhs: Decimal) -> Self::Output {
        Value {
            amount: self.amount / rhs,
            commodity: self.commodity.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
enum InnerValue {
    #[default]
    Zero,

    // By construction, the amount is never zero
    One(Value),

    // We use a hashmap though it might be slightly less efficient than a vector
    // in our code (more memory usage, possibly slower search).  But it provides
    // convenient PartialEq whereas a vector would depend on the order of
    // commodities.
    // By construction, the amount is never zero (or we would have removed the
    // entry altogether).  Also the hashmap always contains at least two
    // elements.
    // We do not use a Commodity as key, since it is mutable.
    Multi(HashMap<CommodityId, Value>),
}

impl MultiValue {
    ///  Return a zero value (currency doesn't matter)
    pub fn zero() -> Self {
        MultiValue(InnerValue::Zero)
    }

    /// A value in one commodity
    pub fn new(amount: Decimal, commodity: &Commodity) -> Self {
        if amount.is_zero() {
            MultiValue::zero()
        } else {
            MultiValue(InnerValue::One(Value {
                amount,
                commodity: commodity.clone(),
            }))
        }
    }

    /// True if the amount is zero for all commodities
    pub fn is_zero(&self) -> bool {
        matches!(self.0, InnerValue::Zero)
    }

    /// If there is a single commodity used in this value, return it.
    pub fn commodity(&self) -> Option<Commodity> {
        match &self.0 {
            InnerValue::Zero => None,
            InnerValue::One(pair) => Some(pair.commodity.clone()),
            InnerValue::Multi(_) => None,
        }
    }

    /// Multiply the amount for a given commodity by the given ratio
    pub fn split(&mut self, commodity: &Commodity, ratio: Decimal) {
        match &mut self.0 {
            InnerValue::Zero => {}
            InnerValue::One(pair) => {
                if pair.commodity == *commodity {
                    pair.amount *= ratio;
                } else {
                    // the amount was zero for this commodity, so still zero
                }
            }
            InnerValue::Multi(map) => {
                if let Some(v) = map.get_mut(&commodity.get_id()) {
                    v.amount *= ratio;
                } else {
                    // the amount was zero for this commodity, so still zero
                }
            }
        }
    }

    /// Iterate over all commodities of the value, skipping zero.
    // ??? Requires malloc for the return type, can we remove iter()
    pub fn iter(&self) -> Box<dyn Iterator<Item = Value> + '_> {
        match &self.0 {
            InnerValue::Zero => Box::new(std::iter::empty()),
            InnerValue::One(pair) => Box::new(std::iter::once(pair.clone())),
            InnerValue::Multi(map) => {
                Box::new(map.values().filter(|v| !v.amount.is_zero()).cloned())
            }
        }
    }

    /// Normalize self:
    /// * Multi variant must contain at least two non-zero values
    /// * One variant must contain a non-zero amount
    fn normalize(&mut self) {
        match &mut self.0 {
            InnerValue::Zero => {}
            InnerValue::One(pair) => {
                if pair.amount.is_zero() {
                    *self = MultiValue(InnerValue::Zero);
                }
            }
            InnerValue::Multi(map) => {
                map.retain(|_, v| !v.amount.is_zero());
                match map.len() {
                    0 => {
                        *self = MultiValue(InnerValue::Zero);
                    }
                    1 => {
                        let p = map.values().next().unwrap();
                        *self = MultiValue(InnerValue::One(p.clone()));
                    }
                    _ => {}
                }
            }
        }
    }

    /// Whether self is properly normalized.
    fn is_normalized(&self) -> bool {
        match &self.0 {
            InnerValue::Zero => true,
            InnerValue::One(pair) => !pair.amount.is_zero(),
            InnerValue::Multi(map) => {
                map.len() >= 2 && map.values().all(|v| !v.amount.is_zero())
            }
        }
    }

    pub fn apply(&mut self, op: &Operation) {
        match op {
            Operation::Credit(value) => {
                *self += value;
            }
            Operation::AddShares { qty }
            | Operation::BuyAmount { qty, .. }
            | Operation::BuyPrice { qty, .. } => {
                *self += qty;
            }

            Operation::Reinvest { shares, .. } => {
                *self += shares;
            }
            Operation::Split { ratio, commodity } => {
                self.split(commodity, *ratio);
            }
            Operation::Dividend => {}
        };
        self.normalize();
    }

    pub fn display(&self, format: &Formatter) -> String {
        let mut into = String::new();
        self.display_into(&mut into, format);
        into
    }

    pub fn display_into(&self, into: &mut String, format: &Formatter) {
        match &self.0 {
            InnerValue::Zero => format.push_zero(into),
            InnerValue::One(pair) => {
                format.push_from_commodity(into, pair.amount, &pair.commodity)
            }
            InnerValue::Multi(map) => {
                for (idx, v) in map.values().enumerate() {
                    if idx > 0 {
                        into.push_str(" + ");
                    }
                    format.push_from_commodity(into, v.amount, &v.commodity);
                }
            }
        }
    }
}

impl core::ops::Div<&MultiValue> for &MultiValue {
    type Output = Option<Decimal>;

    fn div(self, rhs: &MultiValue) -> Self::Output {
        assert!(self.is_normalized());
        assert!(rhs.is_normalized());
        match (&self.0, &rhs.0) {
            (_, InnerValue::Zero) => None,
            (InnerValue::Zero, _) => Some(Decimal::ZERO),
            (InnerValue::One(p1), InnerValue::One(p2)) => {
                Some(p1.amount / p2.amount)
            }
            (_, InnerValue::Multi(_)) => None,
            (InnerValue::Multi(_), _) => None,
        }
    }
}

impl core::ops::Div<Decimal> for &MultiValue {
    type Output = MultiValue;

    fn div(self, rhs: Decimal) -> Self::Output {
        assert!(self.is_normalized());
        match &self.0 {
            InnerValue::Zero => MultiValue::zero(),
            InnerValue::One(p1) => MultiValue(InnerValue::One(p1 / rhs)),
            InnerValue::Multi(m1) => {
                let mut map = m1.clone();
                for v in map.values_mut() {
                    v.amount /= rhs;
                }
                MultiValue(InnerValue::Multi(map))
            }
        }
    }
}

impl core::ops::Div<Decimal> for MultiValue {
    type Output = MultiValue;

    fn div(self, rhs: Decimal) -> Self::Output {
        &self / rhs
    }
}

impl core::ops::Div<&MultiValue> for MultiValue {
    type Output = Option<Decimal>;

    fn div(self, rhs: &MultiValue) -> Self::Output {
        &self / rhs
    }
}

impl core::ops::Div<MultiValue> for &MultiValue {
    type Output = Option<Decimal>;

    fn div(self, rhs: MultiValue) -> Self::Output {
        self / &rhs
    }
}

impl core::ops::Div<MultiValue> for MultiValue {
    type Output = Option<Decimal>;

    fn div(self, rhs: MultiValue) -> Self::Output {
        &self / &rhs
    }
}

impl core::ops::Add<&MultiValue> for &MultiValue {
    type Output = MultiValue;

    fn add(self, rhs: &MultiValue) -> Self::Output {
        assert!(self.is_normalized());
        assert!(rhs.is_normalized());
        match (&self.0, &rhs.0) {
            (InnerValue::Zero, _) => rhs.clone(),
            (_, InnerValue::Zero) => self.clone(),
            (InnerValue::One(p1), InnerValue::One(p2)) => {
                if p1.commodity == p2.commodity {
                    let amount = p1.amount + p2.amount;
                    if amount.is_zero() {
                        MultiValue::zero()
                    } else {
                        MultiValue(InnerValue::One(Value {
                            amount,
                            commodity: p1.commodity.clone(),
                        }))
                    }
                } else {
                    let mut map = HashMap::new();
                    map.insert(p1.commodity.get_id(), p1.clone());
                    map.insert(p2.commodity.get_id(), p2.clone());
                    MultiValue(InnerValue::Multi(map))
                }
            }
            (InnerValue::One(p1), InnerValue::Multi(m2)) => {
                let mut map = m2.clone();
                map.entry(p1.commodity.get_id())
                    .and_modify(|v| v.amount += p1.amount)
                    .or_insert(p1.clone());
                let mut result = MultiValue(InnerValue::Multi(map));

                // ??? We might have cloned m2 for nothing, could be
                // optimized in the future.
                result.normalize();
                result
            }
            (InnerValue::Multi(m1), InnerValue::One(p2)) => {
                let mut map = m1.clone();
                map.entry(p2.commodity.get_id())
                    .and_modify(|v| v.amount += p2.amount)
                    .or_insert(p2.clone());
                let mut result = MultiValue(InnerValue::Multi(map));
                result.normalize();
                result
            }
            (InnerValue::Multi(m1), InnerValue::Multi(m2)) => {
                let mut map = m1.clone();
                for (c2, a2) in m2 {
                    map.entry(*c2)
                        .and_modify(|v| v.amount += a2.amount)
                        .or_insert(a2.clone());
                }
                let mut result = MultiValue(InnerValue::Multi(map));
                result.normalize();
                result
            }
        }
    }
}

impl core::ops::Add<MultiValue> for MultiValue {
    type Output = MultiValue;

    fn add(self, rhs: MultiValue) -> Self::Output {
        self + &rhs
    }
}

impl core::ops::Add<&MultiValue> for MultiValue {
    type Output = MultiValue;

    fn add(self, rhs: &MultiValue) -> Self::Output {
        &self + rhs
    }
}

impl core::ops::Add<MultiValue> for &MultiValue {
    type Output = MultiValue;

    fn add(self, rhs: MultiValue) -> Self::Output {
        self + &rhs
    }
}

impl core::ops::Add<&Value> for &MultiValue {
    type Output = MultiValue;

    fn add(self, rhs: &Value) -> Self::Output {
        self + MultiValue(InnerValue::One(rhs.clone()))
    }
}

impl core::ops::AddAssign<MultiValue> for MultiValue {
    fn add_assign(&mut self, rhs: MultiValue) {
        *self += &rhs;
    }
}

impl core::ops::AddAssign<&Value> for MultiValue {
    fn add_assign(&mut self, rhs: &Value) {
        *self += MultiValue(InnerValue::One(rhs.clone()));
    }
}

impl core::ops::AddAssign<Value> for MultiValue {
    fn add_assign(&mut self, rhs: Value) {
        *self += MultiValue(InnerValue::One(rhs));
    }
}

impl core::ops::AddAssign<&MultiValue> for MultiValue {
    fn add_assign(&mut self, rhs: &MultiValue) {
        assert!(self.is_normalized());
        assert!(rhs.is_normalized());
        match (&mut self.0, &rhs.0) {
            (InnerValue::Zero, _) => {
                *self = rhs.clone();
            }
            (_, InnerValue::Zero) => {}
            (InnerValue::One(p1), InnerValue::One(p2)) => {
                if p1.commodity == p2.commodity {
                    let amount = p1.amount + p2.amount;
                    if amount.is_zero() {
                        *self = MultiValue::zero();
                    } else {
                        p1.amount += p2.amount;
                    }
                } else {
                    let mut map = HashMap::new();
                    map.insert(p1.commodity.get_id(), p1.clone());
                    map.insert(p2.commodity.get_id(), p2.clone());
                    *self = MultiValue(InnerValue::Multi(map));
                }
            }
            (InnerValue::One(p1), InnerValue::Multi(m2)) => {
                let mut map = m2.clone();
                map.entry(p1.commodity.get_id())
                    .and_modify(|v| v.amount += p1.amount)
                    .or_insert(p1.clone());
                *self = MultiValue(InnerValue::Multi(map));
                self.normalize();
            }
            (InnerValue::Multi(m1), InnerValue::One(p2)) => {
                m1.entry(p2.commodity.get_id())
                    .and_modify(|v| v.amount += p2.amount)
                    .or_insert(p2.clone());
                self.normalize();
            }
            (InnerValue::Multi(m1), InnerValue::Multi(m2)) => {
                for (c2, a2) in m2 {
                    m1.entry(*c2)
                        .and_modify(|v| v.amount += a2.amount)
                        .or_insert(a2.clone());
                }
                self.normalize();
            }
        }
    }
}

impl core::ops::Neg for &Value {
    type Output = Value;

    fn neg(self) -> Self::Output {
        Value {
            commodity: self.commodity.clone(),
            amount: -self.amount,
        }
    }
}

impl core::ops::Neg for &MultiValue {
    type Output = MultiValue;

    fn neg(self) -> Self::Output {
        match &self.0 {
            InnerValue::Zero => MultiValue::zero(),
            InnerValue::One(pair) => MultiValue(InnerValue::One(Value {
                amount: -pair.amount,
                commodity: pair.commodity.clone(),
            })),
            InnerValue::Multi(map) => {
                let mut m = map.clone();
                m.values_mut().for_each(|v| v.amount = -v.amount);
                MultiValue(InnerValue::Multi(m))
            }
        }
    }
}

impl core::ops::Neg for MultiValue {
    type Output = MultiValue;

    fn neg(self) -> Self::Output {
        -&self
    }
}

impl core::ops::Sub<&MultiValue> for MultiValue {
    type Output = MultiValue;

    fn sub(self, rhs: &MultiValue) -> Self::Output {
        &self - rhs
    }
}

impl core::ops::Sub<MultiValue> for MultiValue {
    type Output = MultiValue;

    fn sub(self, rhs: MultiValue) -> Self::Output {
        &self - &rhs
    }
}

impl core::ops::Sub<MultiValue> for &MultiValue {
    type Output = MultiValue;

    fn sub(self, rhs: MultiValue) -> Self::Output {
        self - &rhs
    }
}

impl core::ops::Sub<&MultiValue> for &MultiValue {
    type Output = MultiValue;

    fn sub(self, rhs: &MultiValue) -> Self::Output {
        match (&self.0, &rhs.0) {
            (InnerValue::Zero, _) => -rhs,
            (_, InnerValue::Zero) => self.clone(),
            (InnerValue::One(p1), InnerValue::One(p2)) => {
                if p1.commodity == p2.commodity {
                    let amount = p1.amount - p2.amount;
                    if amount.is_zero() {
                        MultiValue::zero()
                    } else {
                        MultiValue(InnerValue::One(Value {
                            amount,
                            commodity: p1.commodity.clone(),
                        }))
                    }
                } else {
                    let mut map = HashMap::new();
                    map.insert(p1.commodity.get_id(), p1.clone());
                    map.insert(p2.commodity.get_id(), (-p2).clone());
                    MultiValue(InnerValue::Multi(map))
                }
            }
            (InnerValue::One(p1), InnerValue::Multi(m2)) => {
                let mut map = HashMap::new();
                map.insert(p1.commodity.get_id(), p1.clone());
                for (c2, a2) in m2 {
                    map.entry(*c2)
                        .and_modify(|v| v.amount -= a2.amount)
                        .or_insert((-a2).clone());
                }
                let mut result = MultiValue(InnerValue::Multi(map));
                result.normalize();
                result
            }
            (InnerValue::Multi(m1), InnerValue::One(p2)) => {
                let mut map = m1.clone();
                map.entry(p2.commodity.get_id())
                    .and_modify(|v| v.amount -= p2.amount)
                    .or_insert(p2.clone());
                let mut result = MultiValue(InnerValue::Multi(map));
                result.normalize();
                result
            }
            (InnerValue::Multi(m1), InnerValue::Multi(m2)) => {
                let mut map = m1.clone();
                for (c2, a2) in m2 {
                    map.entry(*c2)
                        .and_modify(|v| v.amount -= a2.amount)
                        .or_insert(a2.clone());
                }
                let mut result = MultiValue(InnerValue::Multi(map));
                result.normalize();
                result
            }
        }
    }
}

impl core::ops::SubAssign<Value> for MultiValue {
    fn sub_assign(&mut self, rhs: Value) {
        *self -= MultiValue(InnerValue::One(rhs));
    }
}

impl core::ops::SubAssign<&Value> for MultiValue {
    fn sub_assign(&mut self, rhs: &Value) {
        *self -= MultiValue(InnerValue::One(rhs.clone()));
    }
}

impl core::ops::SubAssign<MultiValue> for MultiValue {
    fn sub_assign(&mut self, rhs: MultiValue) {
        *self -= &rhs;
    }
}

impl core::ops::SubAssign<&MultiValue> for MultiValue {
    fn sub_assign(&mut self, rhs: &MultiValue) {
        match (&mut self.0, &rhs.0) {
            (InnerValue::Zero, _) => {
                *self = -rhs;
            }
            (_, InnerValue::Zero) => {}
            (InnerValue::One(p1), InnerValue::One(p2)) => {
                if p1.commodity == p2.commodity {
                    let amount = p1.amount - p2.amount;
                    if amount.is_zero() {
                        *self = MultiValue::zero()
                    } else {
                        p1.amount -= p2.amount;
                    }
                } else {
                    let mut map = HashMap::new();
                    map.insert(p1.commodity.get_id(), p1.clone());
                    map.insert(p2.commodity.get_id(), (-p2).clone());
                    *self = MultiValue(InnerValue::Multi(map));
                }
            }
            (InnerValue::One(p1), InnerValue::Multi(m2)) => {
                let mut map = HashMap::new();
                map.insert(p1.commodity.get_id(), p1.clone());
                for (c2, a2) in m2 {
                    map.entry(*c2)
                        .and_modify(|v| v.amount -= a2.amount)
                        .or_insert((-a2).clone());
                }
                *self = MultiValue(InnerValue::Multi(map));
                self.normalize();
            }
            (InnerValue::Multi(m1), InnerValue::One(p2)) => {
                m1.entry(p2.commodity.get_id())
                    .and_modify(|v| v.amount -= p2.amount)
                    .or_insert(p2.clone());
                self.normalize();
            }
            (InnerValue::Multi(m1), InnerValue::Multi(m2)) => {
                for (c2, a2) in m2 {
                    m1.entry(*c2)
                        .and_modify(|v| v.amount -= a2.amount)
                        .or_insert(a2.clone());
                }
                self.normalize();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::commodities::CommodityCollection;
    use crate::multi_values::MultiValue;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    #[test]
    fn test_add() {
        let mut coms = CommodityCollection::default();
        let c1 = coms.add_dummy("c1", false);
        let c2 = coms.add_dummy("c2", false);
        let one_c1 = MultiValue::new(Decimal::ONE, &c1);
        let two_c1 = MultiValue::new(dec!(2.0), &c1);
        let minus_one_c1 = MultiValue::new(-Decimal::ONE, &c1);
        let one_c2 = MultiValue::new(Decimal::ONE, &c2);

        assert_eq!(MultiValue::zero(), MultiValue::zero(),);
        assert_eq!(MultiValue::zero(), MultiValue::new(Decimal::ZERO, &c1));
        assert_eq!(one_c1, one_c1);
        assert_ne!(one_c1, minus_one_c1);
        assert_eq!(-&one_c1, minus_one_c1);
        assert_eq!(-&minus_one_c1, one_c1);
        assert_eq!(&one_c1 + &minus_one_c1, MultiValue::zero());
        assert_eq!(&one_c1 - &one_c1, MultiValue::zero());
        assert_eq!(&one_c1 + &one_c1, two_c1);

        let mut one_c1_2 = one_c1.clone();
        one_c1_2 += &minus_one_c1;
        assert_eq!(one_c1_2, MultiValue::zero());

        let mut one_c1_2 = one_c1.clone();
        one_c1_2 += &one_c1;
        assert_eq!(one_c1_2, two_c1);

        let mut zero = MultiValue::zero();
        zero += &one_c1;
        assert_eq!(zero, one_c1);

        let mut zero = MultiValue::zero();
        zero += &one_c1;
        zero += &one_c2;
        assert_ne!(zero, MultiValue::zero());
        zero -= &one_c1;
        assert_eq!(zero, one_c2);
        zero -= &one_c2;
        assert_eq!(zero, MultiValue::zero());
    }
}
