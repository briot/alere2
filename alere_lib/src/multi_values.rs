use crate::commodities::{CommodityCollection, CommodityId};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Operation {
    // The amount of the transaction, as seen on the bank statement.
    // This could be a number of shares when the account is a Stock account, for
    // instance, or a number of EUR for a checking account.
    Credit(Value),

    // Buying shares
    Buy(Value),

    // Reinvest dividends and buy shares
    Reinvest(Value),

    // There were some dividends for one of the stocks   The amount will be
    // visible in other splits.
    Dividend(Value),

    // Used for stock splits.  The number of shares is multiplied by the ratio,
    // and their value divided by the same ratio.
    Split {
        ratio: Decimal,
        commodity: CommodityId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Value {
    pub value: Decimal,
    pub commodity: CommodityId,
}

impl Value {
    pub fn new(value: Decimal, commodity: CommodityId) -> Self {
        Value { value, commodity }
    }

    pub fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    pub fn display(&self, commodities: &CommodityCollection) -> String {
        commodities
            .get(self.commodity)
            .unwrap()
            .display(&self.value)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MultiValue {
    values: HashMap<CommodityId, Decimal>,
}

impl MultiValue {
    pub fn from_value(value: Value) -> Self {
        let mut result = MultiValue::default();
        if !value.is_zero() {
            result += value;
        }
        result
    }

    pub fn is_zero(&self) -> bool {
        self.values.iter().all(|(_, v)| v.is_zero())
    }

    pub fn display(&self, commodities: &CommodityCollection) -> String {
        let mut result = String::new();
        for c in &self.values {
            if *c.1 != Decimal::ZERO {
                if !result.is_empty() {
                    result.push_str(" + ");
                }
                result.push_str(&commodities.get(*c.0).unwrap().display(c.1));
            }
        }
        result
    }

    /// Iterate over all components of the multi-value, skipping
    /// zero.
    pub fn iter(&self) -> impl Iterator<Item = Value> + '_ {
        self.values
            .iter()
            .filter(|(_, v)| !v.is_zero())
            .map(|(c, v)| Value {
                value: *v,
                commodity: *c,
            })
    }

    pub fn apply(&mut self, op: &Operation) {
        match op {
            Operation::Credit(value) => {
                *self += *value;
            }
            Operation::Buy(shares) | Operation::Reinvest(shares) => {
                *self += *shares;
            }
            Operation::Split { ratio, commodity } => {
                let mut v = self.values.get_mut(commodity).unwrap();
                v *= ratio;
            }
            Operation::Dividend(value) => {
                *self += *value;
            }
        };
    }
}

impl core::ops::Div<&MultiValue> for &MultiValue {
    type Output = Option<Decimal>;

    fn div(self, rhs: &MultiValue) -> Self::Output {
        let mut s = self.iter();
        let mut t = rhs.iter();
        match s.next() {
            None => None,
            Some(s1) => {
                if s.next().is_some() {
                    None
                } else {
                    match t.next() {
                        None => None,
                        Some(t1) => {
                            if t.next().is_some()
                                || t1.commodity != s1.commodity
                            {
                                None
                            } else {
                                Some(s1.value / t1.value)
                            }
                        }
                    }
                }
            }
        }
    }
}

impl core::ops::Add<Value> for MultiValue {
    type Output = Self;

    fn add(self, rhs: Value) -> Self::Output {
        let mut result = self.clone();
        result += rhs;
        result
    }
}

impl core::ops::Add<&MultiValue> for &MultiValue {
    type Output = MultiValue;

    fn add(self, rhs: &MultiValue) -> Self::Output {
        let mut result = self.clone();
        result += rhs;
        result
    }
}

impl core::ops::Add<&MultiValue> for MultiValue {
    type Output = MultiValue;

    fn add(self, rhs: &MultiValue) -> Self::Output {
        let mut result = self.clone();
        result += rhs;
        result
    }
}

impl core::ops::AddAssign<Value> for MultiValue {
    fn add_assign(&mut self, rhs: Value) {
        *self += &rhs;
    }
}

impl core::ops::AddAssign<&Value> for MultiValue {
    fn add_assign(&mut self, rhs: &Value) {
        if !rhs.is_zero() {
            self.values
                .entry(rhs.commodity)
                .and_modify(|v| *v += rhs.value)
                .or_insert(rhs.value);
        }
    }
}

impl core::ops::AddAssign<&MultiValue> for MultiValue {
    fn add_assign(&mut self, rhs: &MultiValue) {
        for (c, value) in &rhs.values {
            self.values
                .entry(*c)
                .and_modify(|v| *v += *value)
                .or_insert(*value);
        }
    }
}

impl core::ops::Sub<&MultiValue> for &MultiValue {
    type Output = MultiValue;

    fn sub(self, rhs: &MultiValue) -> Self::Output {
        let mut result = self.clone();
        result -= rhs;
        result
    }
}

impl core::ops::SubAssign<&MultiValue> for MultiValue {
    fn sub_assign(&mut self, rhs: &MultiValue) {
        for (c, value) in &rhs.values {
            self.values
                .entry(*c)
                .and_modify(|v| *v -= *value)
                .or_insert(*value);
        }
    }
}
