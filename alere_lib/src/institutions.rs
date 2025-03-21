use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
pub struct InstitutionCollection {
    insts: Vec<Institution>,
}

impl InstitutionCollection {
    pub fn add(
        &mut self,
        name: &str,
        manager: Option<&str>,
        street: Option<&str>,
        zip: Option<&str>,
        city: Option<&str>,
        phone: Option<&str>,
    ) -> Institution {
        let inst = Institution(Rc::new(RefCell::new(InstitutionDetails {
            name: name.into(),
            _manager: manager.map(|s| s.into()),
            _street: street.map(|s| s.into()),
            _zip: zip.map(|s| s.into()),
            _city: city.map(|s| s.into()),
            _phone: phone.map(|s| s.into()),
            icon: None,
            bic: None,
            url: None,
        })));
        self.insts.push(inst.clone());
        inst
    }
}

#[derive(Clone, Debug)]
pub struct Institution(Rc<RefCell<InstitutionDetails>>);

impl Institution {
    pub fn set_icon(self, icon: String) -> Self {
        self.0.borrow_mut().icon = Some(icon);
        self
    }

    pub fn cmp_name(&self, right: &Institution) -> std::cmp::Ordering {
        self.0.borrow().name.cmp(&right.0.borrow().name)
    }

    pub fn set_bic(&mut self, bic: &str) {
        self.0.borrow_mut().bic = Some(bic.to_string());
    }

    pub fn set_url(&mut self, url: &str) {
        self.0.borrow_mut().url = Some(url.to_string());
    }

    pub fn get_name(&self) -> String {
        self.0.borrow().name.clone()
    }
}

impl PartialEq for Institution {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.as_ptr(), other.0.as_ptr())
    }
}

impl Eq for Institution {}

#[derive(Debug)]
pub struct InstitutionDetails {
    name: String, // Display name
    _manager: Option<String>,
    _street: Option<String>,
    _zip: Option<String>,
    _city: Option<String>,
    _phone: Option<String>,
    icon: Option<String>, // URL to the icon
    bic: Option<String>,
    url: Option<String>,
}
