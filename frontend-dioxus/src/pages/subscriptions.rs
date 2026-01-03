use crate::api::{types::SubscriptionResponse, ApiClient};
use crate::hooks::use_auth;
use dioxus::prelude::*;

#[component]
pub fn Subscriptions() -> Element {
    let auth = use_auth();
    let mut subscriptions = use_signal(|| Vec::<SubscriptionResponse>::new());
    let mut loading = use_signal(|| true);

    // Load subscriptions
    let token = auth.token();
    use_effect(move || {
        let token_clone = token.clone();
        spawn(async move {
            if let Some(t) = token_clone {
                let client = ApiClient::new().with_token(Some(t));
                if let Ok(subs) = client.get_subscriptions().await {
                    subscriptions.set(subs);
                }
            }
            loading.set(false);
        });
    });

    let auth_token = auth.token();

    rsx! {
        main { class: "min-h-screen bg-gray-900 py-12",
            div { class: "container mx-auto px-6",
                div { class: "text-center mb-12",
                    h1 { class: "text-4xl md:text-5xl font-bold text-gray-100 mb-6", "My Subscriptions" }
                    p { class: "text-xl text-gray-300 max-w-3xl mx-auto",
                        "Manage packages you're following to receive updates"
                    }
                }

                div { class: "max-w-4xl mx-auto",
                    if loading() {
                        div { class: "flex justify-center py-12",
                            div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" }
                        }
                    } else if subscriptions().is_empty() {
                        div { class: "text-center py-12",
                            div { class: "bg-gray-800 rounded-2xl p-12 border border-gray-700",
                                svg { class: "w-16 h-16 text-gray-600 mx-auto mb-4", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" }
                                }
                                h3 { class: "text-xl font-semibold text-gray-300 mb-2", "No Subscriptions Yet" }
                                p { class: "text-gray-400 mb-6", "Start following packages to get updates on new versions and changes" }
                                Link {
                                    to: crate::Route::Packages {},
                                    class: "inline-block px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors",
                                    "Browse Packages"
                                }
                            }
                        }
                    } else {
                        div { class: "space-y-4",
                            for sub in subscriptions().iter() {
                                {
                                    let pkg_name = sub.package_name.clone();
                                    let pkg_name_toggle = sub.package_name.clone();
                                    let pkg_name_unsub = sub.package_name.clone();
                                    let notif_enabled = sub.notifications_enabled;
                                    let token_toggle = auth_token.clone();
                                    let token_unsub = auth_token.clone();
                                    rsx! {
                                        div { key: "{pkg_name}", class: "bg-gray-800 rounded-xl p-6 border border-gray-700",
                                            div { class: "flex justify-between items-start",
                                                div { class: "flex-1",
                                                    h3 { class: "text-xl font-bold text-gray-100 mb-2",
                                                        "{pkg_name}"
                                                    }
                                                    if let Some(package) = &sub.package {
                                                        if let Some(description) = &package.description {
                                                            p { class: "text-gray-400 text-sm mb-4", "{description}" }
                                                        }
                                                        if let Some(repository) = &package.repository {
                                                            a {
                                                                href: "{repository}",
                                                                target: "_blank",
                                                                class: "text-blue-400 hover:text-blue-300 text-sm",
                                                                "View Repository"
                                                            }
                                                        }
                                                    }
                                                }
                                                div { class: "flex items-center space-x-4",
                                                    label { class: "flex items-center space-x-2 cursor-pointer",
                                                        input {
                                                            r#type: "checkbox",
                                                            class: "w-4 h-4 text-blue-600 bg-gray-700 border-gray-600 rounded focus:ring-blue-500",
                                                            checked: notif_enabled,
                                                            onchange: move |evt| {
                                                                let pkg = pkg_name_toggle.clone();
                                                                let token = token_toggle.clone();
                                                                let enabled = evt.checked();
                                                                spawn(async move {
                                                                    if let Some(t) = token {
                                                                        let client = ApiClient::new().with_token(Some(t));
                                                                        let _ = client.toggle_notifications(&pkg, enabled).await;
                                                                    }
                                                                });
                                                            }
                                                        }
                                                        span { class: "text-sm text-gray-300", "Email Notifications" }
                                                    }
                                                    button {
                                                        class: "px-4 py-2 bg-red-500 hover:bg-red-600 text-white rounded-lg transition-colors",
                                                        onclick: move |_| {
                                                            let pkg = pkg_name_unsub.clone();
                                                            let token = token_unsub.clone();
                                                            spawn(async move {
                                                                if let Some(t) = token {
                                                                    let client = ApiClient::new().with_token(Some(t));
                                                                    if client.unsubscribe(&pkg).await.is_ok() {
                                                                        subscriptions.write().retain(|s| s.package_name != pkg);
                                                                    }
                                                                }
                                                            });
                                                        },
                                                        "Unsubscribe"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
