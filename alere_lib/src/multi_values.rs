use crate::commodities::{CommodityCollection, CommodityId};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Operation {
    // The amount of the transaction, as seen on the bank statement.
    // This could be a number of shares when the account is a Stock account, for
    // instance, or a number of EUR for a checking account.
    Credit(MultiValue),

    // Buying shares
    Buy(MultiValue),

    // Reinvest dividends and buy shares
    Reinvest(MultiValue),

    // There were some dividends for one of the stocks   The amount will be
    // visible in other splits.
    Dividend(MultiValue),

    // Used for stock splits.  The number of shares is multiplied by the ratio,
    // and their value divided by the same ratio.
    Split {
        ratio: Decimal,
        commodity: CommodityId,
    },
}

/// A value includes zero or more pairs (amount, commodity) to express prices
/// or account balances.
/// It can represent values like   1 EUR + 3 USD

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MultiValue(InnerValue);

/// A value is for a single commodity

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Value {
    pub amount: Decimal,
    pub commodity: CommodityId,
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
    Multi(HashMap<CommodityId, Decimal>),
}

impl MultiValue {
    ///  Return a zero value (currency doesn't matter)
    pub fn zero() -> Self {
        MultiValue(InnerValue::Zero)
    }

    /// A value in one commodity
    pub fn new(amount: Decimal, commodity: CommodityId) -> Self {
        if amount.is_zero() {
            MultiValue::zero()
        } else {
            MultiValue(InnerValue::One(Value { amount, commodity }))
        }
    }

    /// True if the amount is zero for all commodities
    pub fn is_zero(&self) -> bool {
        matches!(self.0, InnerValue::Zero)
    }

    /// Iterate over all commodities of the value, skipping zero.
    // ??? Requires malloc for the return type, can we remove iter()
    pub fn iter(&self) -> Box<dyn Iterator<Item = Value> + '_> {
        match &self.0 {
            InnerValue::Zero => Box::new(std::iter::empty()),
            InnerValue::One(pair) => Box::new(std::iter::once(*pair)),
            InnerValue::Multi(map) => {
                Box::new(map.iter().filter(|(_, v)| !v.is_zero()).map(
                    |(c, v)| Value {
                        amount: *v,
                        commodity: *c,
                    },
                ))
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
                map.retain(|_, v| !v.is_zero());
                match map.len() {
                    0 => {
                        *self = MultiValue(InnerValue::Zero);
                    }
                    1 => {
                        let p = map.iter().next().unwrap();
                        *self = MultiValue(InnerValue::One(Value {
                            commodity: *p.0,
                            amount: *p.1,
                        }));
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
                map.len() >= 2 && map.values().all(|v| !v.is_zero())
            }
        }
    }

    pub fn apply(&mut self, op: &Operation) {
        match op {
            Operation::Credit(value) => {
                *self += value;
            }
            Operation::Buy(shares) | Operation::Reinvest(shares) => {
                *self += shares;
            }
            Operation::Split { ratio, commodity } => match &mut self.0 {
                InnerValue::Zero => {}
                InnerValue::One(pair) => {
                    if pair.commodity == *commodity {
                        pair.amount *= ratio;
                    }
                }
                InnerValue::Multi(map) => {
                    if let Some(v) = map.get_mut(commodity) {
                        *v *= ratio;
                    }
                }
            },
            Operation::Dividend(value) => {
                *self += value;
            }
        };
        self.normalize();
    }

    pub fn display(&self, commodities: &CommodityCollection) -> String {
        match &self.0 {
            InnerValue::Zero => "0".to_string(),
            InnerValue::One(pair) => commodities
                .get(pair.commodity)
                .unwrap()
                .display(&pair.amount),
            InnerValue::Multi(map) => {
                let mut s = String::new();
                for (c, v) in map {
                    if !s.is_empty() {
                        s.push_str(" + ");
                    }
                    s.push_str(&commodities.get(*c).unwrap().display(v));
                }
                s
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
                if p1.commodity == p2.commodity {
                    Some(p1.amount / p2.amount)
                } else {
                    None
                }
            }
            (_, InnerValue::Multi(_)) => None,
            (InnerValue::Multi(_), _) => None,
        }
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
                            commodity: p1.commodity,
                        }))
                    }
                } else {
                    let mut map = HashMap::new();
                    map.insert(p1.commodity, p1.amount);
                    map.insert(p2.commodity, p2.amount);
                    MultiValue(InnerValue::Multi(map))
                }
            }
            (InnerValue::One(p1), InnerValue::Multi(m2)) => {
                let mut map = m2.clone();
                map.entry(p1.commodity)
                    .and_modify(|v| *v += p1.amount)
                    .or_insert(p1.amount);
                let mut result = MultiValue(InnerValue::Multi(map));

                // ??? We might have cloned m2 for nothing, could be
                // optimized in the future.
                result.normalize();
                result
            }
            (InnerValue::Multi(m1), InnerValue::One(p2)) => {
                let mut map = m1.clone();
                map.entry(p2.commodity)
                    .and_modify(|v| *v += p2.amount)
                    .or_insert(p2.amount);
                let mut result = MultiValue(InnerValue::Multi(map));
                result.normalize();
                result
            }
            (InnerValue::Multi(m1), InnerValue::Multi(m2)) => {
                let mut map = m1.clone();
                for (c2, a2) in m2 {
                    map.entry(*c2).and_modify(|v| *v += *a2).or_insert(*a2);
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

impl core::ops::AddAssign<MultiValue> for MultiValue {
    fn add_assign(&mut self, rhs: MultiValue) {
        *self += &rhs;
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
                    map.insert(p1.commodity, p1.amount);
                    map.insert(p2.commodity, p2.amount);
                    *self = MultiValue(InnerValue::Multi(map));
                }
            }
            (InnerValue::One(p1), InnerValue::Multi(m2)) => {
                let mut map = m2.clone();
                map.entry(p1.commodity)
                    .and_modify(|v| *v += p1.amount)
                    .or_insert(p1.amount);
                *self = MultiValue(InnerValue::Multi(map));
                self.normalize();
            }
            (InnerValue::Multi(m1), InnerValue::One(p2)) => {
                m1.entry(p2.commodity)
                    .and_modify(|v| *v += p2.amount)
                    .or_insert(p2.amount);
                self.normalize();
            }
            (InnerValue::Multi(m1), InnerValue::Multi(m2)) => {
                for (c2, a2) in m2 {
                    m1.entry(*c2).and_modify(|v| *v += *a2).or_insert(*a2);
                }
                self.normalize();
            }
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
                commodity: pair.commodity,
            })),
            InnerValue::Multi(map) => {
                let mut m = map.clone();
                m.values_mut().for_each(|v| *v = -*v);
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
                            commodity: p1.commodity,
                        }))
                    }
                } else {
                    let mut map = HashMap::new();
                    map.insert(p1.commodity, p1.amount);
                    map.insert(p2.commodity, -p2.amount);
                    MultiValue(InnerValue::Multi(map))
                }
            }
            (InnerValue::One(p1), InnerValue::Multi(m2)) => {
                let mut map = HashMap::new();
                map.insert(p1.commodity, p1.amount);
                for (c2, a2) in m2 {
                    map.entry(*c2).and_modify(|v| *v -= a2).or_insert(-a2);
                }
                let mut result = MultiValue(InnerValue::Multi(map));
                result.normalize();
                result
            }
            (InnerValue::Multi(m1), InnerValue::One(p2)) => {
                let mut map = m1.clone();
                map.entry(p2.commodity)
                    .and_modify(|v| *v -= p2.amount)
                    .or_insert(p2.amount);
                let mut result = MultiValue(InnerValue::Multi(map));
                result.normalize();
                result
            }
            (InnerValue::Multi(m1), InnerValue::Multi(m2)) => {
                let mut map = m1.clone();
                for (c2, a2) in m2 {
                    map.entry(*c2).and_modify(|v| *v -= *a2).or_insert(*a2);
                }
                let mut result = MultiValue(InnerValue::Multi(map));
                result.normalize();
                result
            }
        }
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
                    map.insert(p1.commodity, p1.amount);
                    map.insert(p2.commodity, -p2.amount);
                    *self = MultiValue(InnerValue::Multi(map));
                }
            }
            (InnerValue::One(p1), InnerValue::Multi(m2)) => {
                let mut map = HashMap::new();
                map.insert(p1.commodity, p1.amount);
                for (c2, a2) in m2 {
                    map.entry(*c2).and_modify(|v| *v -= a2).or_insert(-a2);
                }
                *self = MultiValue(InnerValue::Multi(map));
                self.normalize();
            }
            (InnerValue::Multi(m1), InnerValue::One(p2)) => {
                m1.entry(p2.commodity)
                    .and_modify(|v| *v -= p2.amount)
                    .or_insert(p2.amount);
                self.normalize();
            }
            (InnerValue::Multi(m1), InnerValue::Multi(m2)) => {
                for (c2, a2) in m2 {
                    m1.entry(*c2).and_modify(|v| *v -= *a2).or_insert(*a2);
                }
                self.normalize();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::commodities::CommodityId;
    use crate::multi_values::MultiValue;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    #[test]
    fn test_add() {
        let c1 = CommodityId(1);
        let c2 = CommodityId(2);
        let one_c1 = MultiValue::new(Decimal::ONE, c1);
        let two_c1 = MultiValue::new(dec!(2.0), c1);
        let minus_one_c1 = MultiValue::new(-Decimal::ONE, c1);
        let one_c2 = MultiValue::new(Decimal::ONE, c2);

        assert_eq!(MultiValue::zero(), MultiValue::zero(),);
        assert_eq!(MultiValue::zero(), MultiValue::new(Decimal::ZERO, c1),);
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
