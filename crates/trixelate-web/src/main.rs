//! The browser page: a picture dropped in, a sheet out, with its model if asked, with nothing sent anywhere.

mod app;

use wasm_bindgen::JsCast;

fn main() {
    console_error_panic_hook::set_once();
    let page = leptos::prelude::document()
        .get_element_by_id("page")
        .and_then(|page| page.dyn_into::<web_sys::HtmlElement>().ok())
        .expect("the page's index.html holds an element with the id page");
    leptos::mount::mount_to(page, app::App).forget();
}
