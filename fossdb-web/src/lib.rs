mod api;
mod components;
mod hooks;
mod pages;

use dioxus::prelude::*;
use dioxus_logger::tracing::{info, Level};

use components::{ComparisonBar, Navigation, NotificationContainer};
use hooks::{use_keyboard_shortcut, KeyPress};
use pages::{ApiDocs, Home, PackageDetail, Packages, Subscriptions};

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Layout)]
        #[route("/")]
        Home {},
        #[route("/packages")]
        Packages {},
        #[route("/packages/:id")]
        PackageDetail { id: String },
        #[route("/subscriptions")]
        Subscriptions {},
        #[route("/api")]
        ApiDocs {},
}

#[component]
fn Layout() -> Element {
    let nav = navigator();

    // Keyboard shortcuts
    use_keyboard_shortcut(
        KeyPress {
            key: "1",
            ctrl: true,
            shift: false,
            alt: false,
        },
        move || {
            nav.push(Route::Home {});
        },
    );

    use_keyboard_shortcut(
        KeyPress {
            key: "2",
            ctrl: true,
            shift: false,
            alt: false,
        },
        move || {
            nav.push(Route::Packages {});
        },
    );

    use_keyboard_shortcut(
        KeyPress {
            key: "3",
            ctrl: true,
            shift: false,
            alt: false,
        },
        move || {
            nav.push(Route::ApiDocs {});
        },
    );

    rsx! {
        Navigation {}
        NotificationContainer {}
        ComparisonBar {}
        Outlet::<Route> {}
    }
}

#[component]
pub fn App() -> Element {
    // Provide all context providers
    use_context_provider(|| Signal::new(hooks::auth::AuthState::default()));
    use_context_provider(|| Signal::new(hooks::NotificationState::default()));
    use_context_provider(|| Signal::new(components::ComparisonState::default()));

    rsx! {
        document::Link { rel: "stylesheet", href: "https://cdn.tailwindcss.com" }
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap"
        }
        style { {include_str!("styles.css")} }

        div { class: "bg-gradient-to-br from-gray-900 to-gray-800 min-h-screen text-white",
            Router::<Route> {}
        }
    }
}

pub fn launch() {
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    dioxus::launch(App);
}
