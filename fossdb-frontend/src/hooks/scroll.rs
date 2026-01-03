use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Event};

pub fn use_scroll_direction() -> Signal<ScrollDirection> {
    let mut direction = use_signal(|| ScrollDirection::None);
    let mut last_scroll = use_signal(|| 0.0);

    use_effect(move || {
        let win = window().expect("no global `window` exists");

        let closure = Closure::wrap(Box::new(move |_: Event| {
            if let Some(w) = window() {
                let current_scroll = w.scroll_y().unwrap_or(0.0);
                let last = last_scroll();

                if current_scroll > last && current_scroll > 100.0 {
                    direction.set(ScrollDirection::Down);
                } else if current_scroll < last {
                    direction.set(ScrollDirection::Up);
                }

                last_scroll.set(current_scroll);
            }
        }) as Box<dyn FnMut(Event)>);

        win.add_event_listener_with_callback("scroll", closure.as_ref().unchecked_ref())
            .expect("failed to add event listener");

        closure.forget();
    });

    direction
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ScrollDirection {
    Up,
    Down,
    None,
}
