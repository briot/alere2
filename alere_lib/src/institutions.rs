#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Default)]
pub struct InstitutionId(pub u8);

impl InstitutionId {
    pub fn inc(&self) -> Self {
        InstitutionId(self.0 + 1)
    }
}

#[derive(Debug)]
pub struct Institution {
    pub name: String, // Display name
    _manager: Option<String>,
    _street: Option<String>,
    _zip: Option<String>,
    _city: Option<String>,
    _phone: Option<String>,
    icon: Option<String>, // URL to the icon
    pub(crate) bic: Option<String>,
    pub(crate) url: Option<String>,
}

impl Institution {
    pub fn new(
        name: &str,
        manager: Option<&str>,
        street: Option<&str>,
        zip: Option<&str>,
        city: Option<&str>,
        phone: Option<&str>,
    ) -> Self {
        Institution {
            name: name.into(),
            _manager: manager.map(|s| s.into()),
            _street: street.map(|s| s.into()),
            _zip: zip.map(|s| s.into()),
            _city: city.map(|s| s.into()),
            _phone: phone.map(|s| s.into()),
            icon: None,
            bic: None,
            url: None,
        }
    }

    pub fn set_icon(mut self, icon: String) -> Self {
        self.icon = Some(icon);
        self
    }
}
