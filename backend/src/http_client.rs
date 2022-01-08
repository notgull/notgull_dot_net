// GNU AGPL v3 License

use once_cell::sync::Lazy;
use reqwest::Client;

pub static CLIENT: Lazy<Client> = Lazy::new(|| {
    let builder = Client::builder();

    #[cfg(debug_assertions)]
    let builder = builder.danger_accept_invalid_certs(true);

    builder.build().expect("Failed to build `reqwest` client")
});
