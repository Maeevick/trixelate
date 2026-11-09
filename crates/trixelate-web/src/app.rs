//! The page's parts: the picture dropped in, the settings, the preview and the download.
//! Everything happens here, in the browser: no picture is ever sent anywhere.

use std::sync::Arc;
use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;
use trixelate::grid::GridSize;
use trixelate::layout::{Margin, NoRoom, Orientation, PageFormat, most_cells_across};
use trixelate::palette::ColourCount;
use trixelate::picture::Picture;
use trixelate::warning::warnings_for;
use trixelate::{
    Order, fits_on_its_page, make_preview, make_sheet, picture_file, png_file, sheet_file_name,
};
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, Element, File, HtmlAnchorElement, HtmlInputElement, Url};

const REPOSITORY: &str = "https://github.com/Maeevick/trixelate";
/// The id of the line saying why the download is disabled, for its button to point to.
const DOWNLOAD_REFUSED: &str = "download-refused";

/// A picture dropped in: its file's name, its pixels, and the link the page shows it from.
#[derive(Clone)]
struct Dropped {
    name: String,
    picture: Arc<Picture>,
    shown_from: String,
}

/// What the sheet is asked to be, as the form sets it.
#[derive(Clone, Copy, PartialEq)]
struct Settings {
    width: u32,
    colours: usize,
    format: PageFormat,
    landscape: bool,
    margin: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            width: 48,
            colours: 8,
            format: PageFormat::A4,
            landscape: false,
            margin: Margin::AROUND_A_SHEET,
        }
    }
}

/// The order for the sheet asked for, once sure its page has room for it, or why it would be refused.
fn ready(picture: &Picture, settings: Settings) -> Result<Order, String> {
    let most = most_cells_across(settings.format);
    if settings.width > most {
        return Err(beyond(settings.format, most));
    }
    let grid = GridSize::keeping_the_proportions_of(picture, settings.width)
        .map_err(|_| "A grid needs at least one cell across.".to_string())?;
    let colours = ColourCount::new(settings.colours).map_err(|_| {
        "A sheet holds from 2 to 24 colours, like a big box of crayons.".to_string()
    })?;
    let margin = Margin::new(settings.margin).map_err(|_| {
        "A margin from 5 mm, or the title and the credit fall off the page, to 20 mm, or the cells shrink.".to_string()
    })?;
    let order = Order {
        grid,
        colours,
        format: settings.format,
        orientation: if settings.landscape {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        },
        margin,
    };
    fits_on_its_page(picture, order).map_err(explain)?;
    Ok(order)
}

/// How a sheet or its preview is drawn from a picture, its order and the name it is titled with.
type Making = fn(&Picture, Order, &str) -> Result<Picture, NoRoom>;

/// The sheet ordered, or its preview, as PNG bytes, both titled with the name the sheet is saved under so that only
/// their cells differ, or why it cannot be made.
fn paint(picture: &Picture, name: &str, order: Order, make: Making) -> Result<Vec<u8>, String> {
    let sheet = make(picture, order, &sheet_file_name(name, &order)).map_err(explain)?;
    png_file::write(&sheet, order.format.dots_per_inch())
        .map_err(|_| "The sheet could not be turned into a PNG. That one is on us.".to_string())
}

fn beyond(format: PageFormat, most: u32) -> String {
    let paper = format.name();
    format!(
        "An {paper} sheet holds at most {most} cells across, or little fingers give up. Fewer cells, or bigger paper?"
    )
}

fn explain(no_room: NoRoom) -> String {
    match no_room {
        NoRoom::ForTheNumbersInTheCells => {
            "The cells got too small for their numbers. Fewer cells across, or bigger paper?"
        }
    }
    .to_string()
}

/// A text started with a capital, as a sentence on the page.
fn sentence(text: &str) -> String {
    let mut characters = text.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().chain(characters).collect())
        .unwrap_or_default()
}

/// A link to PNG bytes held by the browser, for an image to show or a file to save.
fn png_link(bytes: &[u8]) -> Option<String> {
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
    let options = BlobPropertyBag::new();
    options.set_type("image/png");
    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options).ok()?;
    Url::create_object_url_with_blob(&blob).ok()
}

/// Lets the browser forget a link once nothing shows or saves from it any more.
fn forget(link: &str) {
    let _ = Url::revoke_object_url(link);
}

/// Hands PNG bytes to the browser as a file to save under a name.
fn save(bytes: &[u8], name: &str) {
    let Some(link) = png_link(bytes) else {
        return;
    };
    let anchor = document()
        .create_element("a")
        .ok()
        .and_then(|element| element.dyn_into::<HtmlAnchorElement>().ok());
    if let Some(anchor) = anchor {
        anchor.set_href(&link);
        anchor.set_download(name);
        anchor.click();
    }
    set_timeout(move || forget(&link), Duration::from_secs(10));
}

async fn bytes_of(file: &File) -> Option<Vec<u8>> {
    let buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
        .await
        .ok()?;
    Some(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// Shows PNG bytes from a new link, letting the browser forget the link shown before.
fn show(shown: RwSignal<Option<String>>, png: &[u8]) {
    let previous = shown.get_untracked();
    shown.set(png_link(png));
    if let Some(previous) = previous {
        forget(&previous);
    }
}

/// How far across an element a pointer is, from 0 at its left edge to 100 at its right edge.
fn across(element: &Element, client_x: i32) -> f64 {
    let edges = element.get_bounding_client_rect();
    if edges.width() <= 0.0 {
        return 0.0;
    }
    ((f64::from(client_x) - edges.left()) / edges.width() * 100.0).clamp(0.0, 100.0)
}

/// Runs heavy work after the browser has painted the page once more, so a "busy" note shows first.
fn after_the_next_paint(work: impl FnOnce() + 'static) {
    request_animation_frame(move || request_animation_frame(work));
}

#[component]
pub fn App() -> impl IntoView {
    let dropped = RwSignal::new(None::<Dropped>);
    let unreadable = RwSignal::new(false);
    let hovering = RwSignal::new(false);
    let settings = RwSignal::new(Settings::default());
    let sheet_shown = RwSignal::new(None::<String>);
    let coloured_in_shown = RwSignal::new(None::<String>);
    let coloured_in = RwSignal::new(0.0_f64);
    let sliding = RwSignal::new(false);
    let painting = RwSignal::new(false);
    let refusal = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);
    let latest = StoredValue::new(0_u64);

    let take = move |file: File| {
        let name = file.name();
        spawn_local(async move {
            let picture = bytes_of(&file)
                .await
                .and_then(|bytes| picture_file::read(&bytes).ok());
            let shown_from = Url::create_object_url_with_blob(&file).ok();
            match (picture, shown_from) {
                (Some(picture), Some(shown_from)) => {
                    if let Some(previous) = dropped.get_untracked() {
                        forget(&previous.shown_from);
                    }
                    unreadable.set(false);
                    dropped.set(Some(Dropped {
                        name,
                        picture: Arc::new(picture),
                        shown_from,
                    }));
                }
                _ => unreadable.set(true),
            }
        });
    };

    Effect::new(move |_| {
        let Some((picture, name)) = dropped.with(|dropped| {
            dropped
                .as_ref()
                .map(|d| (d.picture.clone(), d.name.clone()))
        }) else {
            return;
        };
        let settings = settings.get();
        let this = latest.get_value() + 1;
        latest.set_value(this);
        refusal.set(None);
        painting.set(true);
        after_the_next_paint(move || {
            if latest.get_value() != this {
                return;
            }
            let painted = ready(&picture, settings).and_then(|order| {
                Ok((
                    paint(&picture, &name, order, make_sheet)?,
                    paint(&picture, &name, order, make_preview)?,
                ))
            });
            match painted {
                Ok((sheet, preview)) => {
                    show(sheet_shown, &sheet);
                    show(coloured_in_shown, &preview);
                    refusal.set(None);
                }
                Err(why) => refusal.set(Some(why)),
            }
            painting.set(false);
        });
    });

    let download = move || {
        let Some(dropped) = dropped.get_untracked() else {
            return;
        };
        let settings = settings.get_untracked();
        saving.set(true);
        after_the_next_paint(move || {
            let painted = ready(&dropped.picture, settings).and_then(|order| {
                Ok((
                    paint(&dropped.picture, &dropped.name, order, make_sheet)?,
                    order,
                ))
            });
            match painted {
                Ok((png, order)) => save(&png, &sheet_file_name(&dropped.name, &order)),
                Err(why) => refusal.set(Some(why)),
            }
            saving.set(false);
        });
    };

    let file_picked = move |event: leptos::ev::Event| {
        let input: HtmlInputElement = event_target(&event);
        if let Some(file) = input.files().and_then(|files| files.get(0)) {
            take(file);
        }
        input.set_value("");
    };

    let file_dropped = move |event: leptos::ev::DragEvent| {
        event.prevent_default();
        hovering.set(false);
        let file = event
            .data_transfer()
            .and_then(|transfer| transfer.files())
            .and_then(|files| files.get(0));
        if let Some(file) = file {
            take(file);
        }
    };

    let warning = move || {
        let settings = settings.get();
        ColourCount::new(settings.colours)
            .ok()
            .and_then(|colours| warnings_for(settings.width, colours).first().copied())
    };
    let no_picture = move || dropped.with(Option::is_none);
    let slide = move |event: &leptos::ev::PointerEvent| {
        if let Some(element) = event
            .current_target()
            .and_then(|target| target.dyn_into::<Element>().ok())
        {
            coloured_in.set(across(&element, event.client_x()));
        }
    };
    let refused = Memo::new(move |_| {
        let picture = dropped.with(|dropped| dropped.as_ref().map(|d| d.picture.clone()))?;
        ready(&picture, settings.get()).err()
    });

    view! {
        <section
            aria-label="Your picture and how to turn it into a sheet"
            class="grid gap-6 rounded-3xl bg-white p-5 shadow-sm ring-1 ring-stone-200 lg:grid-cols-[1fr_1.4fr]"
        >
            <label
                class=move || {
                    format!(
                        "flex cursor-pointer flex-col items-center justify-center gap-2 rounded-2xl border-2 border-dashed px-6 py-10 text-center transition focus-within:ring-4 focus-within:ring-lime-600 {}",
                        if hovering.get() {
                            "border-lime-500 bg-lime-50"
                        } else {
                            "border-stone-300 hover:border-lime-400 hover:bg-lime-50/50"
                        },
                    )
                }
                on:dragover=move |event: leptos::ev::DragEvent| {
                    event.prevent_default();
                    hovering.set(true);
                }
                on:dragleave=move |_| hovering.set(false)
                on:drop=file_dropped
            >
                <input
                    type="file"
                    accept="image/png,image/jpeg"
                    class="sr-only"
                    on:change=file_picked
                />
                <span aria-hidden="true" class="text-4xl">"🧌"</span>
                <span class="text-lg font-semibold">
                    {move || {
                        if no_picture() {
                            "Drop a picture here".to_string()
                        } else {
                            "Drop another one to swap".to_string()
                        }
                    }}
                </span>
                <span class="text-sm text-stone-500">
                    "or click to pick one · PNG or JPEG"
                </span>
                <Show when=move || unreadable.get()>
                    <span role="alert" class="text-sm font-medium text-rose-600">
                        "That does not look like a PNG or a JPEG. Goblins only come in those two flavours."
                    </span>
                </Show>
            </label>

            <form
                class="grid grid-cols-2 content-center gap-x-5 gap-y-4 sm:grid-cols-3"
                on:submit=|event| event.prevent_default()
            >
                <label class="flex flex-col gap-1 text-sm font-medium">
                    "Cells across"
                    <input
                        type="number"
                        min="1"
                        max=move || {
                            most_cells_across(settings.get().format)
                                .to_string()
                        }
                        class="rounded-xl border border-stone-300 px-3 py-2 font-normal focus:border-lime-600 focus:outline-none focus:ring-2 focus:ring-lime-600"
                        prop:value=move || settings.get().width.to_string()
                        on:change=move |event| {
                            if let Ok(width) = event_target_value(&event).parse::<u32>() {
                                settings.update(|settings| settings.width = width.max(1));
                            }
                        }
                    />
                </label>
                <label class="flex flex-col gap-1 text-sm font-medium">
                    <span>
                        "Colours "
                        <span class="font-mono text-lime-700">
                            {move || settings.get().colours}
                        </span>
                    </span>
                    <input
                        type="range"
                        min="2"
                        max="24"
                        class="mt-2 accent-lime-600"
                        prop:value=move || settings.get().colours.to_string()
                        on:input=move |event| {
                            if let Ok(colours) = event_target_value(&event).parse::<usize>() {
                                settings.update(|settings| settings.colours = colours);
                            }
                        }
                    />
                </label>
                <Show when=move || warning().is_some()>
                    <p
                        role="status"
                        class="col-span-full rounded-2xl bg-amber-100 px-4 py-2 text-sm text-amber-900 ring-1 ring-amber-200"
                    >
                        <span aria-hidden="true">"⚠️ "</span>
                        {move || warning().map(|warning| sentence(warning.says()))}
                    </p>
                </Show>
                <label class="flex flex-col gap-1 text-sm font-medium">
                    "Paper"
                    <select
                        class="rounded-xl border border-stone-300 bg-white px-3 py-2 font-normal focus:border-lime-600 focus:outline-none focus:ring-2 focus:ring-lime-600"
                        on:change=move |event| {
                            let format = match event_target_value(&event).as_str() {
                                "a0" => PageFormat::A0,
                                "a1" => PageFormat::A1,
                                "a2" => PageFormat::A2,
                                "a3" => PageFormat::A3,
                                "a5" => PageFormat::A5,
                                _ => PageFormat::A4,
                            };
                            settings.update(|settings| settings.format = format);
                        }
                    >
                        <optgroup label="Sheets, 300 dpi">
                            <option value="a3">"A3"</option>
                            <option value="a4" selected>"A4"</option>
                            <option value="a5">"A5"</option>
                        </optgroup>
                        <optgroup label="Posters, 150 dpi">
                            <option value="a2">"A2"</option>
                            <option value="a1">"A1"</option>
                            <option value="a0">"A0"</option>
                        </optgroup>
                    </select>
                </label>
                <label class="flex flex-col gap-1 text-sm font-medium">
                    "Margin, in mm"
                    <input
                        type="number"
                        min="5"
                        max="20"
                        class="rounded-xl border border-stone-300 px-3 py-2 font-normal focus:border-lime-600 focus:outline-none focus:ring-2 focus:ring-lime-600"
                        prop:value=move || settings.get().margin.to_string()
                        on:change=move |event| {
                            if let Ok(margin) = event_target_value(&event).parse::<u32>() {
                                settings.update(|settings| settings.margin = margin);
                            }
                        }
                    />
                </label>
                <label class="flex items-start gap-2 text-sm font-medium sm:mt-6">
                    <input
                        type="checkbox"
                        class="mt-0.5 size-4 accent-lime-600"
                        prop:checked=move || settings.get().landscape
                        on:change=move |event| {
                            let landscape = event_target_checked(&event);
                            settings.update(|settings| settings.landscape = landscape);
                        }
                    />
                    <span>
                        "Landscape"
                        <span class="block font-normal text-stone-500">"for wide pictures"</span>
                    </span>
                </label>
            </form>
            <div
                role="group"
                aria-label="Download"
                class="grid gap-3 border-t border-stone-100 pt-5 sm:flex sm:flex-wrap sm:items-center sm:justify-center lg:col-span-2"
            >
                <span class="text-center text-sm text-stone-500">
                    {move || if saving.get() { "Sharpening the crayons…" } else { "Download the" }}
                </span>
                <button
                    type="button"
                    disabled=move || saving.get() || no_picture() || refused.get().is_some()
                    title=move || refused.get().unwrap_or_else(|| "to colour in".to_string())
                    aria-describedby=move || refused.get().map(|_| DOWNLOAD_REFUSED.to_string())
                    class="group w-full rounded-full bg-stone-800 px-5 py-2.5 text-sm font-semibold text-amber-50 shadow-sm transition hover:-translate-y-0.5 hover:bg-stone-700 disabled:translate-y-0 disabled:cursor-not-allowed disabled:bg-stone-100 disabled:text-stone-400 disabled:shadow-none sm:w-auto"
                    on:click=move |_| download()
                >
                    "Sheet"
                    <span class="ml-1.5 font-normal text-amber-200/80 group-disabled:text-stone-400">
                        {move || {
                            if refused.get().is_some() { "doesn't fit" } else { "to colour in" }
                        }}
                    </span>
                </button>
                <Show when=move || refused.get().is_some()>
                    <p id=DOWNLOAD_REFUSED class="basis-full text-center text-sm text-stone-600">
                        {move || refused.get()}
                    </p>
                </Show>
            </div>
        </section>

        <section aria-label="Preview" class="grid gap-6 md:grid-cols-2">
            <figure class="flex flex-col gap-3 rounded-3xl bg-white p-5 shadow-sm ring-1 ring-stone-200">
                <figcaption class="flex h-9 items-center justify-between text-sm font-semibold text-stone-600">
                    "Your picture"
                    <span class="font-mono text-xs font-normal text-stone-500">
                        {move || {
                            dropped
                                .get()
                                .map(|dropped| {
                                    format!(
                                        "{} · {} × {}",
                                        dropped.name,
                                        dropped.picture.width(),
                                        dropped.picture.height(),
                                    )
                                })
                                .unwrap_or_default()
                        }}
                    </span>
                </figcaption>
                <div class="flex min-h-80 flex-1 items-center justify-center rounded-2xl bg-stone-100 p-3">
                    {move || match dropped.get() {
                        Some(dropped) => {
                            view! {
                                <img
                                    src=dropped.shown_from
                                    alt="The picture you dropped in"
                                    class="max-h-[70vh] w-full object-contain"
                                />
                            }
                                .into_any()
                        }
                        None => {
                            view! {
                                <p class="text-center text-stone-600">
                                    "No picture yet. The goblin is waiting."
                                </p>
                            }
                                .into_any()
                        }
                    }}
                </div>
            </figure>

            <figure class="flex flex-col gap-3 rounded-3xl bg-white p-5 shadow-sm ring-1 ring-stone-200">
                <figcaption class="flex h-9 items-center justify-between text-sm font-semibold text-stone-600">
                    "The sheet, with its model"
                    <Show when=move || coloured_in_shown.with(Option::is_some) && !no_picture()>
                        <span class="text-xs font-normal text-stone-500">
                            "Slide the line to colour it in"
                        </span>
                    </Show>
                </figcaption>
                <div class="relative flex min-h-80 flex-1 items-center justify-center rounded-2xl bg-stone-100 p-3">
                    {move || match (sheet_shown.get(), no_picture()) {
                        (Some(link), false) => {
                            view! {
                                <div
                                    class=move || {
                                        format!(
                                            "relative w-fit max-w-full cursor-ew-resize touch-pan-y select-none transition {}",
                                            if refusal.get().is_some() { "opacity-25" } else { "" },
                                        )
                                    }
                                    on:pointerdown=move |event| {
                                        if let Some(element) = event
                                            .current_target()
                                            .and_then(|target| target.dyn_into::<Element>().ok())
                                        {
                                            let _ = element.set_pointer_capture(event.pointer_id());
                                        }
                                        sliding.set(true);
                                        slide(&event);
                                    }
                                    on:pointermove=move |event| {
                                        if sliding.get_untracked() {
                                            slide(&event);
                                        }
                                    }
                                    on:pointerup=move |_| sliding.set(false)
                                    on:pointercancel=move |_| sliding.set(false)
                                >
                                    <img
                                        src=link
                                        alt="The sheet: a grid of numbered cells to colour in, its model in miniature in the top left corner"
                                        draggable="false"
                                        class="block max-h-[70vh] w-auto max-w-full drop-shadow-md"
                                    />
                                    {move || {
                                        coloured_in_shown
                                            .get()
                                            .map(|link| {
                                                view! {
                                                    <img
                                                        src=link
                                                        alt=""
                                                        draggable="false"
                                                        class="pointer-events-none absolute inset-0 size-full"
                                                        style=move || {
                                                            format!(
                                                                "clip-path: inset(0 {:.2}% 0 0)",
                                                                100.0 - coloured_in.get(),
                                                            )
                                                        }
                                                    />
                                                }
                                            })
                                    }}
                                    <input
                                        type="range"
                                        min="0"
                                        max="100"
                                        step="5"
                                        aria-label="How much of the sheet is shown coloured in"
                                        aria-valuetext=move || {
                                            format!("{:.0}% coloured in", coloured_in.get())
                                        }
                                        class="peer sr-only"
                                        prop:value=move || format!("{:.0}", coloured_in.get())
                                        on:input=move |event| {
                                            if let Ok(share) = event_target_value(&event).parse::<f64>() {
                                                coloured_in.set(share);
                                            }
                                        }
                                    />
                                    <span
                                        aria-hidden="true"
                                        class="pointer-events-none absolute inset-y-0 w-0.5 -translate-x-1/2 bg-lime-500 shadow-sm"
                                        style=move || format!("left: {:.2}%", coloured_in.get())
                                    ></span>
                                    <span
                                        aria-hidden="true"
                                        class="pointer-events-none absolute top-1/2 grid size-9 -translate-x-1/2 -translate-y-1/2 place-items-center rounded-full bg-stone-800 text-amber-50 shadow-md ring-lime-500 peer-focus-visible:ring-4"
                                        style=move || format!("left: {:.2}%", coloured_in.get())
                                    >
                                        "⇆"
                                    </span>
                                </div>
                            }
                                .into_any()
                        }
                        _ => {
                            view! {
                                <p class="text-center text-stone-600">
                                    "Your sheet shows up here, once there is something to pixelate."
                                </p>
                            }
                                .into_any()
                        }
                    }}
                    <Show when=move || painting.get()>
                        <p
                            role="status"
                            class="absolute top-5 right-5 rounded-full bg-white/90 px-3 py-1 text-xs font-medium text-stone-600 shadow-sm"
                        >
                            "Sharpening the crayons…"
                        </p>
                    </Show>
                    <Show when=move || refusal.get().is_some()>
                        <p
                            role="alert"
                            class="absolute inset-x-5 top-5 rounded-2xl bg-rose-50 px-4 py-3 text-sm font-medium text-rose-700 shadow-sm ring-1 ring-rose-200"
                        >
                            {move || refusal.get().unwrap_or_default()}
                        </p>
                    </Show>
                </div>
            </figure>
        </section>

        <footer class="mt-auto text-right text-xs text-stone-400">
            "Made with trixelate ("
            <a
                href=REPOSITORY
                target="_blank"
                rel="noopener noreferrer"
                class="font-medium text-stone-200 underline decoration-lime-500 decoration-2 underline-offset-2 hover:text-lime-400"
            >
                "github.com/Maeevick/trixelate"
                <span class="sr-only">" (opens in a new tab)"</span>
            </a>
            ") - MIT License"
        </footer>
    }
}
