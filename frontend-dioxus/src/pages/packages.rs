use crate::api::ApiClient;
use crate::components::{PackageCard, use_comparison};
use crate::hooks::{use_auth, LocalStorage, StorageKey};
use crate::api::types::Package;
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct PackageFilters {
    pub search: String,
    pub category: String,
    pub language: String,
    pub license: String,
    pub date_range: String,
    pub sort: String,
}

impl Default for PackageFilters {
    fn default() -> Self {
        Self {
            search: String::new(),
            category: String::new(),
            language: String::new(),
            license: String::new(),
            date_range: String::new(),
            sort: "name".to_string(),
        }
    }
}

#[component]
pub fn Packages() -> Element {
    let auth = use_auth();
    let mut packages = use_signal(|| Vec::<Package>::new());
    let mut filters = use_signal(PackageFilters::default);
    let mut loading = use_signal(|| true);
    let mut show_advanced = use_signal(|| false);
    let mut current_page = use_signal(|| 1u32);
    let mut total_packages = use_signal(|| 0usize);
    let mut total_pages = use_signal(|| 0u32);
    let page_size = 30u32;

    // Load view mode from localStorage
    let mut view_mode = use_signal(|| {
        LocalStorage::get::<String>(StorageKey::ViewMode)
            .unwrap_or_else(|| "grid".to_string())
    });

    let token = auth.token();
    let mut search_trigger = use_signal(|| 0);

    // Search effect - runs when search_trigger or current_page changes
    use_effect(move || {
        let _ = search_trigger(); // Subscribe to changes
        let page = current_page();
        let filter_state = filters();
        let token_clone = token.clone();

        spawn(async move {
            loading.set(true);
            let client = ApiClient::new().with_token(token_clone);

            let query = if filter_state.search.is_empty() {
                None
            } else {
                Some(filter_state.search.clone())
            };

            if let Ok(response) = client.get_packages(query, page, page_size).await {
                let mut pkg_list = response.packages;

                // Apply client-side filters
                if !filter_state.language.is_empty() {
                    pkg_list.retain(|p| {
                        p.language
                            .as_ref()
                            .map(|l| l.eq_ignore_ascii_case(&filter_state.language))
                            .unwrap_or(false)
                    });
                }
                if !filter_state.license.is_empty() {
                    pkg_list.retain(|p| {
                        p.license
                            .as_ref()
                            .map(|l| l.eq_ignore_ascii_case(&filter_state.license))
                            .unwrap_or(false)
                    });
                }

                // Apply sorting
                match filter_state.sort.as_str() {
                    "name" => pkg_list.sort_by(|a, b| a.name.cmp(&b.name)),
                    "-created_at" => pkg_list.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
                    "created_at" => pkg_list.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
                    "-updated_at" => pkg_list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
                    _ => {}
                }

                packages.set(pkg_list);
                total_packages.set(response.total);
                total_pages.set(((response.total as f64) / (page_size as f64)).ceil() as u32);
            }
            loading.set(false);
        });
    });

    let mut perform_search = move || {
        current_page.set(1); // Reset to first page when searching
        search_trigger.set(search_trigger() + 1);
    };

    let mut set_view_mode = move |mode: &str| {
        view_mode.set(mode.to_string());
        let _ = LocalStorage::set(StorageKey::ViewMode, &mode.to_string());
    };

    let mut clear_filters = move |_| {
        filters.set(PackageFilters::default());
        perform_search();
    };

    rsx! {
        main { class: "min-h-screen bg-gray-900 py-12",
            div { class: "container mx-auto px-6",
                div { class: "text-center mb-12",
                    h1 { class: "text-4xl md:text-5xl font-bold text-gray-100 mb-6", "Explore Packages" }
                    p { class: "text-xl text-gray-300 max-w-3xl mx-auto",
                        "Browse our comprehensive collection of open source packages from various ecosystems"
                    }
                }

                // Advanced Search and Filter Section
                div { class: "bg-gray-800 rounded-2xl shadow-xl p-8 mb-8 border border-gray-700",
                    // Main Search Row
                    div { class: "flex flex-col lg:flex-row gap-6 mb-6",
                        div { class: "flex-1",
                            label { class: "block text-sm font-medium text-gray-300 mb-2", "Search Packages" }
                            div { class: "relative",
                                svg {
                                    class: "absolute left-4 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-400",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                                    }
                                }
                                input {
                                    r#type: "text",
                                    placeholder: "Search packages, libraries, frameworks...",
                                    class: "w-full pl-12 pr-4 py-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100 placeholder-gray-400",
                                    value: "{filters().search}",
                                    oninput: move |evt| {
                                        filters.write().search = evt.value();
                                    },
                                    onkeydown: move |evt| {
                                        if evt.key() == Key::Enter {
                                            perform_search();
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "lg:w-48",
                            label { class: "block text-sm font-medium text-gray-300 mb-2", "Sort By" }
                            select {
                                class: "w-full p-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100",
                                value: "{filters().sort}",
                                onchange: move |evt| {
                                    filters.write().sort = evt.value();
                                    perform_search();
                                },
                                option { value: "name", "Name (A-Z)" }
                                option { value: "-created_at", "Newest First" }
                                option { value: "created_at", "Oldest First" }
                                option { value: "-updated_at", "Recently Updated" }
                            }
                        }

                        div { class: "lg:w-auto flex items-end space-x-3",
                            button {
                                class: "px-4 py-3 border border-gray-600 rounded-lg hover:bg-gray-700 transition-colors flex items-center space-x-2 text-gray-300",
                                onclick: move |_| show_advanced.set(!show_advanced()),
                                svg { class: "w-4 h-4", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 100 4m0-4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 100 4m0-4v2m0-6V4" }
                                }
                                span { "Filters" }
                            }

                            // View Mode Toggle
                            div { class: "flex border border-gray-600 rounded-lg overflow-hidden",
                                button {
                                    class: if view_mode() == "grid" { "bg-blue-500 text-white px-3 py-3 transition-colors" } else { "bg-gray-700 text-gray-300 px-3 py-3 transition-colors" },
                                    onclick: move |_| set_view_mode("grid"),
                                    svg { class: "w-4 h-4", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" }
                                    }
                                }
                                button {
                                    class: if view_mode() == "list" { "bg-blue-500 text-white px-3 py-3 transition-colors" } else { "bg-gray-700 text-gray-300 px-3 py-3 transition-colors" },
                                    onclick: move |_| set_view_mode("list"),
                                    svg { class: "w-4 h-4", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M4 6h16M4 10h16M4 14h16M4 18h16" }
                                    }
                                }
                            }
                        }
                    }

                    // Advanced Filters
                    if show_advanced() {
                        div { class: "border-t border-gray-700 pt-6",
                            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4",
                                div {
                                    label { class: "block text-sm font-medium text-gray-300 mb-2", "Category" }
                                    select {
                                        class: "w-full p-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100",
                                        value: "{filters().category}",
                                        onchange: move |evt| {
                                            filters.write().category = evt.value();
                                            perform_search();
                                        },
                                        option { value: "", "All Categories" }
                                        option { value: "web", "Web Development" }
                                        option { value: "cli", "Command Line Tools" }
                                        option { value: "library", "Libraries" }
                                        option { value: "framework", "Frameworks" }
                                        option { value: "database", "Database" }
                                        option { value: "security", "Security" }
                                        option { value: "devtools", "Developer Tools" }
                                    }
                                }

                                div {
                                    label { class: "block text-sm font-medium text-gray-300 mb-2", "Language" }
                                    select {
                                        class: "w-full p-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100",
                                        value: "{filters().language}",
                                        onchange: move |evt| {
                                            filters.write().language = evt.value();
                                            perform_search();
                                        },
                                        option { value: "", "All Languages" }
                                        option { value: "rust", "Rust" }
                                        option { value: "javascript", "JavaScript" }
                                        option { value: "python", "Python" }
                                        option { value: "go", "Go" }
                                        option { value: "java", "Java" }
                                        option { value: "cpp", "C++" }
                                        option { value: "csharp", "C#" }
                                    }
                                }

                                div {
                                    label { class: "block text-sm font-medium text-gray-300 mb-2", "License" }
                                    select {
                                        class: "w-full p-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100",
                                        value: "{filters().license}",
                                        onchange: move |evt| {
                                            filters.write().license = evt.value();
                                            perform_search();
                                        },
                                        option { value: "", "All Licenses" }
                                        option { value: "MIT", "MIT" }
                                        option { value: "Apache-2.0", "Apache 2.0" }
                                        option { value: "GPL-3.0", "GPL 3.0" }
                                        option { value: "BSD-3-Clause", "BSD 3-Clause" }
                                        option { value: "ISC", "ISC" }
                                        option { value: "LGPL-2.1", "LGPL 2.1" }
                                    }
                                }

                                div {
                                    label { class: "block text-sm font-medium text-gray-300 mb-2", "Date Range" }
                                    select {
                                        class: "w-full p-3 bg-gray-700 border border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-400 focus:border-blue-400 text-gray-100",
                                        value: "{filters().date_range}",
                                        onchange: move |evt| {
                                            filters.write().date_range = evt.value();
                                            perform_search();
                                        },
                                        option { value: "", "All Time" }
                                        option { value: "today", "Today" }
                                        option { value: "week", "Past Week" }
                                        option { value: "month", "Past Month" }
                                        option { value: "year", "Past Year" }
                                    }
                                }
                            }

                            div { class: "flex justify-between items-center mt-6 pt-4 border-t border-gray-700",
                                button {
                                    class: "px-4 py-2 text-gray-400 hover:text-gray-200 transition-colors",
                                    onclick: clear_filters,
                                    "Clear All Filters"
                                }
                                div { class: "text-sm text-gray-400",
                                    "Showing {packages().len()} packages"
                                }
                            }
                        }
                    }
                }

                // Packages List
                if loading() {
                    div { class: "flex justify-center py-12",
                        div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" }
                    }
                } else {
                    div {
                        class: if view_mode() == "grid" {
                            "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6"
                        } else {
                            "space-y-4"
                        },
                        for package in packages().iter() {
                            PackageCard { package: package.clone() }
                        }
                    }
                }

                if !loading() && packages().is_empty() {
                    div { class: "text-center py-12",
                        p { class: "text-gray-400 text-lg", "No packages found" }
                    }
                }

                // Pagination controls
                if !loading() && total_pages() > 1 {
                    div { class: "flex justify-center items-center gap-2 mt-8",
                        button {
                            class: "px-4 py-2 bg-gray-700 text-white rounded-lg hover:bg-gray-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: current_page() == 1,
                            onclick: move |_| {
                                current_page.set(current_page() - 1);
                                search_trigger.set(search_trigger() + 1);
                            },
                            "Previous"
                        }

                        div { class: "flex gap-1",
                            for page_num in get_page_range(current_page(), total_pages()) {
                                if page_num == 0 {
                                    span { class: "px-3 py-2 text-gray-400", "..." }
                                } else {
                                    button {
                                        class: if current_page() == page_num {
                                            "px-3 py-2 bg-blue-600 text-white rounded-lg"
                                        } else {
                                            "px-3 py-2 bg-gray-700 text-white rounded-lg hover:bg-gray-600 transition-colors"
                                        },
                                        onclick: move |_| {
                                            current_page.set(page_num);
                                            search_trigger.set(search_trigger() + 1);
                                        },
                                        "{page_num}"
                                    }
                                }
                            }
                        }

                        button {
                            class: "px-4 py-2 bg-gray-700 text-white rounded-lg hover:bg-gray-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: current_page() >= total_pages(),
                            onclick: move |_| {
                                current_page.set(current_page() + 1);
                                search_trigger.set(search_trigger() + 1);
                            },
                            "Next"
                        }

                        div { class: "text-gray-400 text-sm ml-4",
                            "Page {current_page()} of {total_pages()} ({total_packages()} total packages)"
                        }
                    }
                }
            }
        }
    }
}

// Helper function to generate page numbers for pagination
fn get_page_range(current: u32, total: u32) -> Vec<u32> {
    let mut pages = Vec::new();

    if total <= 7 {
        // Show all pages if 7 or fewer
        for i in 1..=total {
            pages.push(i);
        }
    } else {
        // Always show first page
        pages.push(1);

        if current > 3 {
            pages.push(0); // Ellipsis
        }

        // Show pages around current
        let start = (current.saturating_sub(1)).max(2);
        let end = (current + 1).min(total - 1);

        for i in start..=end {
            pages.push(i);
        }

        if current < total - 2 {
            pages.push(0); // Ellipsis
        }

        // Always show last page
        pages.push(total);
    }

    pages
}
