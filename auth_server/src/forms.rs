// GNU AGPL v3 License

use crate::Config;
use once_cell::sync::OnceCell;
use tokio::{
    fs,
    io::{AsyncReadExt, BufReader},
};

const LOGIN_FORM_TEMPLATE: &'static str = include_str!("../forms/login.html");
static FORMS: OnceCell<Forms> = OnceCell::new();

struct Forms {
    login_form: String,
}

#[inline]
pub fn initialize_forms(cfg: &Config) {
    let login_form = LOGIN_FORM_TEMPLATE;
    let login_form = login_form.replace("%%base_url%%", &cfg.urls.base_url);
    let login_form = login_form.replace("%%redirect_url%%", &cfg.verify.redirect_uri);

    let _ = FORMS.set(Forms { login_form });
}

#[inline]
pub fn login_form() -> &'static str {
    &FORMS.get().unwrap().login_form
}
