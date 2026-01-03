use crate::api::types::Package;
use crate::hooks::use_notifications;
use dioxus::prelude::*;
use std::collections::VecDeque;

#[derive(Clone, PartialEq)]
pub struct ComparisonState {
    pub packages: VecDeque<Package>,
}

impl Default for ComparisonState {
    fn default() -> Self {
        Self {
            packages: VecDeque::new(),
        }
    }
}

pub struct ComparisonContext {
    state: Signal<ComparisonState>,
}

impl ComparisonContext {
    pub fn add(&mut self, package: Package) {
        let mut state = self.state.write();
        let mut notif = use_notifications();

        if state.packages.len() >= 3 {
            notif.warning("Maximum 3 packages can be compared".to_string());
            return;
        }

        if state.packages.iter().any(|p| p.id == package.id) {
            notif.info("Package already in comparison".to_string());
            return;
        }

        state.packages.push_back(package);
        notif.success("Package added to comparison".to_string());
    }

    pub fn remove(&mut self, package_id: u64) {
        let mut state = self.state.write();
        state.packages.retain(|p| p.id != package_id);
    }

    pub fn clear(&mut self) {
        self.state.write().packages.clear();
    }

    pub fn count(&self) -> usize {
        self.state.read().packages.len()
    }
}

pub fn use_comparison() -> ComparisonContext {
    let state = use_context::<Signal<ComparisonState>>();
    ComparisonContext { state }
}

#[component]
pub fn ComparisonBar() -> Element {
    let comparison = use_comparison();
    let mut show_modal = use_signal(|| false);

    let packages = comparison.state.read().packages.clone();
    if packages.is_empty() {
        return rsx! { div {} };
    }

    let package_count = packages.len();
    let packages_for_modal: Vec<_> = packages.iter().cloned().collect();

    rsx! {
        div { class: "fixed bottom-4 right-4 bg-gray-800 rounded-lg shadow-2xl border border-gray-700 p-4 z-40",
            div { class: "flex items-center space-x-4",
                div { class: "flex items-center space-x-2",
                    svg { class: "w-5 h-5 text-blue-400", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" }
                    }
                    span { class: "text-gray-200 font-medium", "Compare ({package_count}/3)" }
                }

                for pkg in packages.into_iter() {
                    {
                        let pkg_id = pkg.id;
                        let pkg_name = pkg.name.clone();
                        rsx! {
                            div { class: "flex items-center space-x-2 bg-gray-700 rounded px-3 py-1",
                                span { class: "text-sm text-gray-200", "{pkg_name}" }
                                button {
                                    class: "text-red-400 hover:text-red-300",
                                    onclick: move |_| {
                                        let mut comp = use_comparison();
                                        comp.remove(pkg_id);
                                    },
                                    svg { class: "w-4 h-4", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M6 18L18 6M6 6l12 12" }
                                    }
                                }
                            }
                        }
                    }
                }

                button {
                    class: "px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors",
                    onclick: move |_| show_modal.set(true),
                    "Compare"
                }

                button {
                    class: "px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-500 transition-colors",
                    onclick: move |_| {
                        let mut comp = use_comparison();
                        comp.clear();
                    },
                    "Clear"
                }
            }
        }

        if show_modal() {
            ComparisonModal { show: show_modal, packages: packages_for_modal }
        }
    }
}

#[component]
fn ComparisonModal(show: Signal<bool>, packages: Vec<Package>) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4",
            onclick: move |_| show.set(false),
            div {
                class: "bg-gray-800 rounded-2xl w-full max-w-6xl max-h-[90vh] overflow-y-auto shadow-2xl border border-gray-700",
                onclick: move |evt| evt.stop_propagation(),
                div { class: "p-8",
                    h2 { class: "text-3xl font-bold text-gray-100 mb-6", "Package Comparison" }

                    div { class: "overflow-x-auto",
                        table { class: "w-full",
                            thead {
                                tr { class: "border-b border-gray-700",
                                    th { class: "text-left p-4 text-gray-400 font-medium", "Property" }
                                    for pkg in packages.iter() {
                                        th { class: "text-left p-4 text-gray-200 font-bold", "{pkg.name}" }
                                    }
                                }
                            }
                            tbody {
                                tr { class: "border-b border-gray-700",
                                    td { class: "p-4 text-gray-400", "Description" }
                                    for pkg in packages.iter() {
                                        td { class: "p-4 text-gray-200",
                                            if let Some(desc) = &pkg.description {
                                                "{desc}"
                                            } else {
                                                span { class: "text-gray-500", "N/A" }
                                            }
                                        }
                                    }
                                }
                                tr { class: "border-b border-gray-700",
                                    td { class: "p-4 text-gray-400", "Homepage" }
                                    for pkg in packages.iter() {
                                        td { class: "p-4",
                                            if let Some(homepage) = &pkg.homepage {
                                                a { class: "text-blue-400 hover:underline", href: "{homepage}", target: "_blank", "Visit" }
                                            } else {
                                                span { class: "text-gray-500", "N/A" }
                                            }
                                        }
                                    }
                                }
                                tr { class: "border-b border-gray-700",
                                    td { class: "p-4 text-gray-400", "Repository" }
                                    for pkg in packages.iter() {
                                        td { class: "p-4",
                                            if let Some(repo) = &pkg.repository {
                                                a { class: "text-blue-400 hover:underline", href: "{repo}", target: "_blank", "Visit" }
                                            } else {
                                                span { class: "text-gray-500", "N/A" }
                                            }
                                        }
                                    }
                                }
                                tr { class: "border-b border-gray-700",
                                    td { class: "p-4 text-gray-400", "Created" }
                                    for pkg in packages.iter() {
                                        td { class: "p-4 text-gray-200", "{pkg.created_at.format(\"%Y-%m-%d\")}" }
                                    }
                                }
                            }
                        }
                    }

                    button {
                        class: "mt-6 w-full bg-gray-600 text-gray-200 px-4 py-3 rounded-lg font-medium hover:bg-gray-500 transition-all",
                        onclick: move |_| show.set(false),
                        "Close"
                    }
                }
            }
        }
    }
}
