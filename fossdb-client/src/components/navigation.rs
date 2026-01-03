use crate::components::modals::{LoginModal, RegisterModal};
use crate::hooks::{use_auth, use_scroll_direction, ScrollDirection};
use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn Navigation() -> Element {
    let mut auth = use_auth();
    let mut mobile_open = use_signal(|| false);
    let mut login_modal_open = use_signal(|| false);
    let mut register_modal_open = use_signal(|| false);
    let scroll_direction = use_scroll_direction();
    let notif = crate::hooks::use_notifications();

    let is_authenticated = auth.is_authenticated();
    let username = auth.user().as_ref().map(|u| u.username.clone());

    // Auto-hide navigation on scroll down
    let nav_class = if scroll_direction() == ScrollDirection::Down {
        "backdrop-blur-md bg-gray-900/80 border-b border-gray-700 sticky top-0 z-50 transform -translate-y-full transition-transform duration-300"
    } else {
        "backdrop-blur-md bg-gray-900/80 border-b border-gray-700 sticky top-0 z-50 transform translate-y-0 transition-transform duration-300"
    };

    rsx! {
        LoginModal { show: login_modal_open }
        RegisterModal { show: register_modal_open }

        nav { class: "{nav_class}",
            div { class: "container mx-auto px-6 py-4",
                div { class: "flex justify-between items-center",
                    // Logo
                    div { class: "flex items-center space-x-4",
                        div { class: "relative",
                            div { class: "w-10 h-10 bg-gradient-to-r from-blue-500 to-purple-600 rounded-xl flex items-center justify-center",
                                svg { class: "w-6 h-6 text-white", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2",
                                        d: "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
                                    }
                                }
                            }
                        }
                        h1 { class: "text-2xl font-bold bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent",
                            "FossDB"
                        }
                    }

                    // Desktop Navigation
                    div { class: "hidden md:flex items-center space-x-8",
                        Link { to: Route::Home {}, class: "nav-link text-gray-300 hover:text-blue-400 font-medium transition-colors",
                            "Home"
                        }
                        Link { to: Route::Packages {}, class: "nav-link text-gray-300 hover:text-blue-400 font-medium transition-colors",
                            "Packages"
                        }
                        if is_authenticated {
                            Link { to: Route::Subscriptions {}, class: "nav-link text-gray-300 hover:text-blue-400 font-medium transition-colors",
                                "Subscriptions"
                            }
                        }
                        Link { to: Route::ApiDocs {}, class: "nav-link text-gray-300 hover:text-blue-400 font-medium transition-colors",
                            "API"
                        }

                        // Auth buttons
                        if is_authenticated {
                            div { class: "flex items-center space-x-3",
                                span { class: "text-gray-300 font-medium",
                                    "{username.as_deref().unwrap_or(\"\")}"
                                }
                                button {
                                    class: "px-4 py-2 bg-red-500 hover:bg-red-600 text-white rounded-lg font-medium transition-all",
                                    onclick: move |_| {
                                        auth.logout();
                                        let mut notif_copy = notif;
                                        notif_copy.success("Logged out successfully".to_string());
                                    },
                                    "Logout"
                                }
                            }
                        } else {
                            div { class: "flex items-center space-x-3",
                                button {
                                    class: "px-4 py-2 text-blue-400 hover:bg-gray-700 rounded-lg font-medium transition-all",
                                    onclick: move |_| login_modal_open.set(true),
                                    "Login"
                                }
                                button {
                                    class: "px-6 py-2 bg-gradient-to-r from-blue-500 to-purple-600 text-white rounded-lg font-medium hover:from-blue-600 hover:to-purple-700 transition-all shadow-lg hover:shadow-xl",
                                    onclick: move |_| register_modal_open.set(true),
                                    "Get Started"
                                }
                            }
                        }
                    }

                    // Mobile menu button
                    button {
                        class: "md:hidden p-2 rounded-lg hover:bg-gray-700",
                        onclick: move |_| mobile_open.set(!mobile_open()),
                        svg { class: "w-6 h-6", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M4 6h16M4 12h16M4 18h16" }
                        }
                    }
                }

                // Mobile Navigation
                if mobile_open() {
                    div { class: "md:hidden mt-4 pb-4 border-t border-gray-600",
                        div { class: "flex flex-col space-y-3 pt-4",
                            Link { to: Route::Home {}, class: "nav-link text-gray-300 hover:text-blue-400 font-medium",
                                "Home"
                            }
                            Link { to: Route::Packages {}, class: "nav-link text-gray-300 hover:text-blue-400 font-medium",
                                "Packages"
                            }
                            if is_authenticated {
                                Link { to: Route::Subscriptions {}, class: "nav-link text-gray-300 hover:text-blue-400 font-medium",
                                    "Subscriptions"
                                }
                            }
                            Link { to: Route::ApiDocs {}, class: "nav-link text-gray-300 hover:text-blue-400 font-medium",
                                "API"
                            }

                            if !is_authenticated {
                                button {
                                    class: "text-left text-blue-400 font-medium",
                                    onclick: move |_| login_modal_open.set(true),
                                    "Login"
                                }
                                button {
                                    class: "text-left px-4 py-2 bg-gradient-to-r from-blue-500 to-purple-600 text-white rounded-lg font-medium w-fit",
                                    onclick: move |_| register_modal_open.set(true),
                                    "Get Started"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
