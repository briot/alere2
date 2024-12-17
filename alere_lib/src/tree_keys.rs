use crate::account_kinds::AccountKind;
use crate::accounts::Account;
use crate::institutions::Institution;

/// An enum that can be used as the key for trees.
#[derive(Clone)]
pub enum Key<'a> {
    Account(&'a Account),
    Institution(Option<&'a Institution>),
    AccountKind(Option<&'a AccountKind>),
}

impl PartialEq for Key<'_> {
    fn eq(&self, right: &Self) -> bool {
        match self {
            Key::Account(la) => match right {
                // Compare pointers, since a given account only exists once.
                // Otherwise, two accounts with the same name but different
                // parents would compare equal
                Key::Account(ra) => std::ptr::eq(*la, *ra),
                _ => false,
            },
            Key::Institution(Some(li)) => match right {
                Key::Institution(Some(ri)) => std::ptr::eq(*li, *ri),
                _ => false,
            },
            _ => matches!(self.cmp(right), std::cmp::Ordering::Equal),
        }
    }
}

impl Eq for Key<'_> {}

impl Ord for Key<'_> {
    fn cmp(&self, right: &Self) -> std::cmp::Ordering {
        match self {
            Key::Account(ka) => match right {
                Key::Account(ra) => ka.name.cmp(&ra.name),
                Key::Institution(_) | Key::AccountKind(_) => {
                    std::cmp::Ordering::Greater
                }
            },
            Key::Institution(Some(ki)) => match right {
                Key::Account(_) => std::cmp::Ordering::Less,
                Key::AccountKind(_) => std::cmp::Ordering::Less,
                Key::Institution(Some(ri)) => ki.name.cmp(&ri.name),
                Key::Institution(None) => std::cmp::Ordering::Less,
            },
            Key::Institution(None) => match right {
                Key::Account(_) => std::cmp::Ordering::Less,
                Key::AccountKind(_) => std::cmp::Ordering::Less,
                Key::Institution(Some(_)) => std::cmp::Ordering::Greater,
                Key::Institution(None) => std::cmp::Ordering::Equal,
            },
            Key::AccountKind(None) => match right {
                Key::Account(_) => std::cmp::Ordering::Less,
                Key::Institution(_) => std::cmp::Ordering::Greater,
                Key::AccountKind(Some(_)) => std::cmp::Ordering::Greater,
                Key::AccountKind(None) => std::cmp::Ordering::Equal,
            },
            Key::AccountKind(Some(kk)) => match right {
                Key::Account(_) => std::cmp::Ordering::Less,
                Key::Institution(_) => std::cmp::Ordering::Greater,
                Key::AccountKind(Some(vk)) => kk.name.cmp(&vk.name),
                Key::AccountKind(None) => std::cmp::Ordering::Less,
            },
        }
    }
}

impl PartialOrd for Key<'_> {
    fn partial_cmp(&self, right: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(right))
    }
}

#[cfg(test)]
mod test {
    use crate::account_categories::AccountCategory;
    use crate::account_kinds::{AccountKind, AccountKindId};
    use crate::accounts::Account;
    use crate::institutions::Institution;
    use crate::tree_keys::Key;

    #[test]
    fn test_sort() {
        let kind_id = AccountKindId(1);
        let acc_aaa = Account::new(
            "aaa", kind_id, None, None, None, None, None, false, None,
        );
        let acc_bbb = Account::new(
            "bbb", kind_id, None, None, None, None, None, false, None,
        );
        let inst_ccc = Institution::new("ccc", None, None, None, None, None);
        let inst_ddd = Institution::new("ddd", None, None, None, None, None);
        let kind_eee =
            AccountKind::new("eee", "Inc", "Dec", AccountCategory::EXPENSE);
        let kind_fff =
            AccountKind::new("fff", "Inc", "Dec", AccountCategory::INCOME);

        let key_acc_aaa = Key::Account(&acc_aaa);
        let key_acc_bbb = Key::Account(&acc_bbb);
        let key_inst_ccc = Key::Institution(Some(&inst_ccc));
        let key_inst_ddd = Key::Institution(Some(&inst_ddd));
        let key_inst_none = Key::Institution(None);
        let key_kind_eee = Key::AccountKind(Some(&kind_eee));
        let key_kind_fff = Key::AccountKind(Some(&kind_fff));
        let key_kind_none = Key::AccountKind(None);

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
        assert!(key_kind_eee < key_kind_none);
        assert!(key_kind_none > key_kind_eee);
        assert!(key_kind_eee == key_kind_eee);
        assert!(key_kind_none == key_kind_none);

        assert!(key_inst_ccc < key_acc_aaa);
        assert!(key_inst_none < key_acc_aaa);

        assert!(key_acc_aaa > key_inst_ccc);
        assert!(key_acc_aaa > key_inst_none);

        assert!(key_kind_none < key_acc_aaa);
        assert!(key_acc_aaa > key_kind_none);

        assert!(key_kind_eee < key_acc_aaa);
        assert!(key_acc_aaa > key_kind_eee);

        // Institution always comes from AccountKind
        assert!(key_kind_none > key_inst_ccc);
        assert!(key_inst_ccc < key_kind_none);
        assert!(key_kind_eee > key_inst_ccc);
        assert!(key_inst_ccc < key_kind_eee);
    }
}
