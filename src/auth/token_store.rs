use keyring::Entry;

const SERVICE: &str = "gitlink";
const ACCOUNT: &str = "github";

pub fn save_token(token: &str) -> Result<(), keyring::Error> {
    let entry = Entry::new(SERVICE, ACCOUNT)?;
    entry.set_password(token)
}

pub fn load_token() -> Result<String, keyring::Error> {
    let entry = Entry::new(SERVICE, ACCOUNT)?;
    entry.get_password()
}

pub fn delete_token() -> Result<(), keyring::Error> {
    let entry = Entry::new(SERVICE, ACCOUNT)?;
    entry.delete_password()
}
