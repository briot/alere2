#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy, Default)]
pub struct InstitutionId(pub u8);

impl InstitutionId {
    pub fn inc(&self) -> Self {
        InstitutionId(self.0 + 1)
    }
}

#[derive(Debug)]
pub struct Institution {
    name: String, // Display name
    manager: Option<String>,
    street: Option<String>,
    zip: Option<String>,
    city: Option<String>,
    phone: Option<String>,
    icon: Option<String>, // URL to the icon
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
            manager: manager.map(|s| s.into()),
            street: street.map(|s| s.into()),
            zip: zip.map(|s| s.into()),
            city: city.map(|s| s.into()),
            phone: phone.map(|s| s.into()),
            icon: None,
        }
    }

    pub fn set_icon(mut self, icon: String) -> Self {
        self.icon = Some(icon);
        self
    }
}
