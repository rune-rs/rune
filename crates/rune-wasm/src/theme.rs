//! The colour theme toggle.
//!
//! A stored choice is also applied by a line of script in the page's head, so
//! that it holds from the first paint rather than from when the wasm loads.

use yew::prelude::*;

use crate::icons;

const KEY: &str = "theme";

#[derive(Clone, Copy, PartialEq)]
enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    fn name(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    fn stored() -> Self {
        let value = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(KEY).ok().flatten());

        match value.as_deref() {
            Some("light") => Theme::Light,
            Some("dark") => Theme::Dark,
            _ => Theme::System,
        }
    }

    fn apply(self) {
        let Some(window) = web_sys::window() else {
            return;
        };

        if let Some(root) = window.document().and_then(|d| d.document_element()) {
            let _ = match self {
                Theme::System => root.remove_attribute("data-theme"),
                theme => root.set_attribute("data-theme", theme.name()),
            };
        }

        if let Ok(Some(storage)) = window.local_storage() {
            let _ = match self {
                Theme::System => storage.remove_item(KEY),
                theme => storage.set_item(KEY, theme.name()),
            };
        }
    }
}

#[function_component]
pub fn ThemeToggle() -> Html {
    let theme = use_state(|| Theme::System);

    {
        let theme = theme.clone();
        use_effect_with((), move |_| theme.set(Theme::stored()));
    }

    let options = [
        (Theme::System, "Match the system", icons::system()),
        (Theme::Light, "Light", icons::sun()),
        (Theme::Dark, "Dark", icons::moon()),
    ];

    let options = options.into_iter().map(|(option, label, icon)| {
        let onclick = {
            let theme = theme.clone();

            Callback::from(move |_| {
                option.apply();
                theme.set(option);
            })
        };

        html! {
            <button type="button" class="theme-toggle-option" title={label} aria-label={label}
                aria-pressed={(*theme == option).to_string()} {onclick}>
                {icon}
            </button>
        }
    });

    html! {
        <div class="theme-toggle" role="group" aria-label="Colour theme">
            {for options}
        </div>
    }
}
