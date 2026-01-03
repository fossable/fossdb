use crate::api::{types::*, ApiClient};
use crate::hooks::{use_auth, use_notifications};
use dioxus::prelude::*;

#[component]
pub fn PackageDetail(id: String) -> Element {
    let auth = use_auth();
    let mut package = use_signal(|| None::<Package>);
    let mut versions = use_signal(|| Vec::<PackageVersion>::new());
    let mut displayed_versions = use_signal(|| Vec::<PackageVersion>::new());
    let mut subscribers = use_signal(|| 0usize);
    let mut loading = use_signal(|| true);
    let mut is_subscribed = use_signal(|| false);
    let mut notifications_enabled = use_signal(|| false);
    let mut page_size = use_signal(|| 10);
    let mut current_page = use_signal(|| 0);

    let token = auth.token();
    let package_id = id.clone();
    let is_authenticated = auth.is_authenticated();

    // Load package data
    let token_for_effect = token.clone();
    let package_id_for_effect = package_id.clone();

    use_effect(move || {
        let token_clone = token_for_effect.clone();
        let pkg_id = package_id_for_effect.clone();
        spawn(async move {
            let client = ApiClient::new().with_token(token_clone.clone());

            if let Ok(pkg) = client.get_package(&pkg_id).await {
                package.set(Some(pkg));
            }

            if let Ok(vers) = client.get_package_versions(&pkg_id).await {
                versions.set(vers.clone());
                let end = (page_size() as usize).min(vers.len());
                displayed_versions.set(vers[0..end].to_vec());
            }

            if let Ok(count) = client.get_package_subscribers(&pkg_id).await {
                subscribers.set(count);
            }

            // Check if user is subscribed
            if is_authenticated {
                if let Ok(subs) = client.get_subscriptions().await {
                    if let Some(pkg) = package() {
                        if let Some(sub) = subs.iter().find(|s| s.package_name == pkg.name) {
                            is_subscribed.set(true);
                            notifications_enabled.set(sub.notifications_enabled);
                        }
                    }
                }
            }

            loading.set(false);
        });
    });

    let mut update_pagination = move || {
        let page = current_page();
        let size = page_size();
        let all_vers = versions();
        let start = (page * size) as usize;
        let end = ((page + 1) * size) as usize;
        let end = end.min(all_vers.len());

        if start < all_vers.len() {
            displayed_versions.set(all_vers[start..end].to_vec());
        }
    };

    let mut change_page_size = move |new_size: i32| {
        page_size.set(new_size);
        current_page.set(0);
        update_pagination();
    };

    let next_page = move |_| {
        let total_pages = (versions().len() as f32 / page_size() as f32).ceil() as i32;
        if current_page() < total_pages - 1 {
            current_page.set(current_page() + 1);
            update_pagination();
        }
    };

    let prev_page = move |_| {
        if current_page() > 0 {
            current_page.set(current_page() - 1);
            update_pagination();
        }
    };

    let notif = use_notifications();

    let token_for_subscribe = token.clone();
    let package_id_for_subscribe = package_id.clone();

    let handle_subscribe = move |_| {
        let mut notif_copy = notif;
        let token_clone = token_for_subscribe.clone();
        let pkg_id = package_id_for_subscribe.clone();

        spawn(async move {
            let client = ApiClient::new().with_token(token_clone);

            if is_subscribed() {
                // Unsubscribe
                if let Ok(_) = client.unsubscribe(&pkg_id).await {
                    is_subscribed.set(false);
                    notif_copy.success("Unsubscribed successfully".to_string());
                    // Decrement subscriber count
                    subscribers.set(subscribers().saturating_sub(1));
                } else {
                    notif_copy.error("Failed to unsubscribe".to_string());
                }
            } else {
                // Subscribe
                if let Ok(_) = client.subscribe(pkg_id.clone()).await {
                    is_subscribed.set(true);
                    notifications_enabled.set(true);
                    notif_copy.success("Subscribed successfully".to_string());
                    // Increment subscriber count
                    subscribers.set(subscribers() + 1);
                } else {
                    notif_copy.error("Failed to subscribe".to_string());
                }
            }
        });
    };

    let token_for_toggle = token.clone();
    let package_id_for_toggle = package_id.clone();

    let toggle_notifications = move |_| {
        let mut notif_copy = notif;
        let token_clone = token_for_toggle.clone();
        let pkg_id = package_id_for_toggle.clone();

        spawn(async move {
            let client = ApiClient::new().with_token(token_clone);
            let new_state = !notifications_enabled();

            if let Ok(_) = client.toggle_notifications(&pkg_id, new_state).await {
                notifications_enabled.set(new_state);
                let msg = if new_state {
                    "Notifications enabled"
                } else {
                    "Notifications disabled"
                };
                notif_copy.success(msg.to_string());
            } else {
                notif_copy.error("Failed to update notification settings".to_string());
            }
        });
    };

    let total_pages = (versions().len() as f32 / page_size() as f32).ceil() as i32;

    rsx! {
        main { class: "min-h-screen bg-gray-900 py-12",
            div { class: "container mx-auto px-6",
                div { class: "max-w-5xl mx-auto",
                    // Back Button
                    div { class: "mb-6",
                        Link {
                            to: crate::Route::Packages {},
                            class: "flex items-center space-x-2 text-gray-400 hover:text-blue-400 transition-colors",
                            svg { class: "w-5 h-5", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M15 19l-7-7 7-7" }
                            }
                            span { "Back to Packages" }
                        }
                    }

                    if loading() {
                        div { class: "flex justify-center py-12",
                            div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" }
                        }
                    } else if let Some(pkg) = package() {
                        // Package Header
                        div { class: "bg-gray-800 rounded-2xl shadow-xl p-8 mb-6 border border-gray-700",
                            div { class: "flex justify-between items-start mb-4",
                                h1 { class: "text-4xl font-bold text-gray-100", "{pkg.name}" }
                                if let Some(license) = &pkg.license {
                                    span { class: "px-4 py-2 bg-blue-900 text-blue-300 rounded-full text-sm font-medium",
                                        "{license}"
                                    }
                                }
                            }

                            if let Some(description) = &pkg.description {
                                p { class: "text-gray-300 text-lg mb-6", "{description}" }
                            }

                            div { class: "flex flex-wrap gap-4",
                                if let Some(homepage) = &pkg.homepage {
                                    a {
                                        href: "{homepage}",
                                        target: "_blank",
                                        class: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors",
                                        "Homepage"
                                    }
                                }
                                if let Some(repository) = &pkg.repository {
                                    a {
                                        href: "{repository}",
                                        target: "_blank",
                                        class: "px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors",
                                        "Repository"
                                    }
                                }
                            }
                        }

                        // Package Details Grid
                        div { class: "grid grid-cols-1 lg:grid-cols-3 gap-6",
                            // Main Content - Versions with Pagination
                            div { class: "lg:col-span-2",
                                div { class: "bg-gray-800 rounded-2xl shadow-xl p-8 border border-gray-700",
                                    div { class: "flex justify-between items-center mb-6",
                                        h2 { class: "text-2xl font-bold text-gray-100", "Versions" }

                                        // Page size selector
                                        div { class: "flex items-center space-x-2",
                                            span { class: "text-sm text-gray-400", "Show:" }
                                            select {
                                                class: "p-2 bg-gray-700 border border-gray-600 rounded text-gray-100 text-sm",
                                                value: "{page_size()}",
                                                onchange: move |evt| {
                                                    if let Ok(size) = evt.value().parse::<i32>() {
                                                        change_page_size(size);
                                                    }
                                                },
                                                option { value: "5", "5" }
                                                option { value: "10", "10" }
                                                option { value: "25", "25" }
                                                option { value: "50", "50" }
                                                option { value: "100", "All" }
                                            }
                                        }
                                    }

                                    div { class: "space-y-2",
                                        for version in displayed_versions().iter() {
                                            div { key: "{version.id}", class: "flex justify-between items-center p-4 bg-gray-700 rounded-lg",
                                                div {
                                                    div { class: "font-semibold text-gray-100", "{version.version}" }
                                                    div { class: "text-sm text-gray-400",
                                                        "Released: {version.release_date.format(\"%Y-%m-%d\")}"
                                                    }
                                                }
                                            }
                                        }

                                        if displayed_versions().is_empty() {
                                            div { class: "text-center py-8 text-gray-400",
                                                "No versions available"
                                            }
                                        }
                                    }

                                    // Pagination controls
                                    if total_pages > 1 {
                                        div { class: "flex justify-between items-center mt-6 pt-4 border-t border-gray-700",
                                            button {
                                                class: "px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded disabled:opacity-50 disabled:cursor-not-allowed",
                                                disabled: current_page() == 0,
                                                onclick: prev_page,
                                                "Previous"
                                            }

                                            span { class: "text-gray-400",
                                                "Page {current_page() + 1} of {total_pages}"
                                            }

                                            button {
                                                class: "px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded disabled:opacity-50 disabled:cursor-not-allowed",
                                                disabled: current_page() >= total_pages - 1,
                                                onclick: next_page,
                                                "Next"
                                            }
                                        }
                                    }
                                }
                            }

                            // Sidebar
                            div { class: "space-y-6",
                                // Subscriber Count
                                div { class: "bg-gray-800 rounded-2xl shadow-xl p-6 border border-gray-700",
                                    h3 { class: "text-lg font-bold text-gray-100 mb-4", "Subscribers" }
                                    div { class: "text-center",
                                        div { class: "text-4xl font-bold text-blue-400", "{subscribers()}" }
                                    }
                                }

                                // Subscribe/Notification Actions
                                if is_authenticated {
                                    div { class: "bg-gray-800 rounded-2xl shadow-xl p-6 border border-gray-700",
                                        h3 { class: "text-lg font-bold text-gray-100 mb-4", "Actions" }
                                        div { class: "space-y-3",
                                            button {
                                                class: if is_subscribed() {
                                                    "w-full px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors"
                                                } else {
                                                    "w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
                                                },
                                                onclick: handle_subscribe,
                                                if is_subscribed() { "Unsubscribe" } else { "Subscribe" }
                                            }

                                            if is_subscribed() {
                                                button {
                                                    class: "w-full px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded-lg transition-colors",
                                                    onclick: toggle_notifications,
                                                    if notifications_enabled() {
                                                        "Disable Notifications"
                                                    } else {
                                                        "Enable Notifications"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Package Info
                                div { class: "bg-gray-800 rounded-2xl shadow-xl p-6 border border-gray-700",
                                    h3 { class: "text-lg font-bold text-gray-100 mb-4", "Information" }
                                    div { class: "space-y-3 text-sm",
                                        if let Some(language) = &pkg.language {
                                            div {
                                                div { class: "text-gray-400", "Language" }
                                                div { class: "text-gray-100 font-medium", "{language}" }
                                            }
                                        }
                                        if let Some(platform) = &pkg.platform {
                                            div {
                                                div { class: "text-gray-400", "Platform" }
                                                div { class: "text-gray-100 font-medium", "{platform}" }
                                            }
                                        }
                                        div {
                                            div { class: "text-gray-400", "Created" }
                                            div { class: "text-gray-100 font-medium", "{pkg.created_at.format(\"%Y-%m-%d\")}" }
                                        }
                                        div {
                                            div { class: "text-gray-400", "Updated" }
                                            div { class: "text-gray-100 font-medium", "{pkg.updated_at.format(\"%Y-%m-%d\")}" }
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
