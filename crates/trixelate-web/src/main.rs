//! The browser page: a picture dropped in, a sheet, a model or a twin out, with nothing sent anywhere.

mod app;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
