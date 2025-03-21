use crate::account_kinds::AccountKind;
use crate::accounts::Account;
use crate::institutions::Institution;

/// An enum that can be used as the key for trees.
#[derive(Clone, PartialEq, Eq)]
pub enum Key {
    Account(Account),
    Institution(Option<Institution>),
    AccountKind(AccountKind),
}

impl Ord for Key {
    fn cmp(&self, right: &Self) -> std::cmp::Ordering {
        match self {
            Key::Account(ka) => match right {
                Key::Account(ra) => ka.cmp_name(ra),
                Key::Institution(_) | Key::AccountKind(_) => {
                    std::cmp::Ordering::Greater
                }
            },
            Key::Institution(Some(ki)) => match right {
                Key::Account(_) => std::cmp::Ordering::Less,
                Key::AccountKind(_) => std::cmp::Ordering::Less,
                Key::Institution(Some(ri)) => ki.cmp_name(ri),
                Key::Institution(None) => std::cmp::Ordering::Less,
            },
            Key::Institution(None) => match right {
                Key::Account(_) => std::cmp::Ordering::Less,
                Key::AccountKind(_) => std::cmp::Ordering::Less,
                Key::Institution(Some(_)) => std::cmp::Ordering::Greater,
                Key::Institution(None) => std::cmp::Ordering::Equal,
            },
            Key::AccountKind(kk) => match right {
                Key::Account(_) => std::cmp::Ordering::Less,
                Key::Institution(_) => std::cmp::Ordering::Greater,
                Key::AccountKind(vk) => kk.cmp_name(vk),
            },
        }
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, right: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(right))
    }
}

#[cfg(test)]
mod test {
    use crate::account_categories::AccountCategory;
    use crate::account_kinds::AccountKind;
    use crate::accounts::AccountCollection;
    use crate::institutions::InstitutionCollection;
    use crate::tree_keys::Key;

    #[test]
    fn test_sort() {
        let mut accounts = AccountCollection::default();
        let mut insts = InstitutionCollection::default();
        let kind_eee =
            AccountKind::new("eee", "Inc", "Dec", AccountCategory::EXPENSE);
        let kind_fff =
            AccountKind::new("fff", "Inc", "Dec", AccountCategory::INCOME);
        let acc_aaa = accounts.add_dummy("aaa", kind_eee.clone());
        let acc_bbb = accounts.add_dummy("bbb", kind_eee.clone());
        let inst_ccc = insts.add("ccc", None, None, None, None, None);
        let inst_ddd = insts.add("ddd", None, None, None, None, None);

        let key_acc_aaa = Key::Account(acc_aaa);
        let key_acc_bbb = Key::Account(acc_bbb);
        let key_inst_ccc = Key::Institution(Some(inst_ccc.clone()));
        let key_inst_ddd = Key::Institution(Some(inst_ddd.clone()));
        let key_inst_none = Key::Institution(None);
        let key_kind_eee = Key::AccountKind(kind_eee.clone());
        let key_kind_fff = Key::AccountKind(kind_fff.clone());

        assert!(key_acc_aaa == key_acc_aaa);
        assert!(key_acc_aaa < key_acc_bbb);
        assert!(key_acc_bbb > key_acc_aaa);

        assert!(key_inst_ccc < key_inst_ddd);
        assert!(key_inst_ddd > key_inst_ccc);
        assert!(key_inst_ccc < key_inst_none);
        assert!(key_inst_none > key_inst_ccc);
        assert!(key_inst_ccc == key_inst_ccc);
        assert!(key_inst_none == key_inst_none);

        assert!(key_kind_eee < key_kind_fff);
        assert!(key_kind_fff > key_kind_eee);
        assert!(key_kind_eee == key_kind_eee);

        assert!(key_inst_ccc < key_acc_aaa);
        assert!(key_inst_none < key_acc_aaa);

        assert!(key_acc_aaa > key_inst_ccc);
        assert!(key_acc_aaa > key_inst_none);

        assert!(key_kind_eee < key_acc_aaa);
        assert!(key_acc_aaa > key_kind_eee);

        // Institution always comes from AccountKind
        assert!(key_kind_eee > key_inst_ccc);
        assert!(key_inst_ccc < key_kind_eee);
    }
}
