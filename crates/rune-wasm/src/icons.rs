//! Inline icons, drawn on a 24 unit grid in the current text colour.

use yew::prelude::*;

fn stroked(paths: Html) -> Html {
    html! {
        <svg class="icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            {paths}
        </svg>
    }
}

pub fn sun() -> Html {
    stroked(html! {
        <>
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
        </>
    })
}

pub fn moon() -> Html {
    stroked(html! { <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" /> })
}

pub fn system() -> Html {
    stroked(html! {
        <>
        <circle cx="12" cy="12" r="9" />
        <path d="M12 3a9 9 0 0 1 0 18z" fill="currentColor" />
        </>
    })
}

pub fn book() -> Html {
    stroked(html! {
        <path d="M2 4h6a4 4 0 0 1 4 4v13a3 3 0 0 0-3-3H2zM22 4h-6a4 4 0 0 0-4 4v13a3 3 0 0 1 3-3h7z" />
    })
}

pub fn chat() -> Html {
    stroked(html! {
        <path d="M21 11.5a8.4 8.4 0 0 1-9 8.5 8.4 8.4 0 0 1-3.8-.9L3 21l1.9-5.2A8.4 8.4 0 0 1 4 11.5 8.5 8.5 0 0 1 12.5 3h.5a8.5 8.5 0 0 1 8 8z" />
    })
}

pub fn rss() -> Html {
    stroked(html! {
        <>
        <path d="M4 11a9 9 0 0 1 9 9M4 4a16 16 0 0 1 16 16" />
        <circle cx="5" cy="19" r="1" />
        </>
    })
}

pub fn play() -> Html {
    stroked(html! { <path d="M6 4l14 8-14 8z" /> })
}

pub fn arrow() -> Html {
    stroked(html! { <path d="M5 12h14M13 6l6 6-6 6" /> })
}

pub fn github() -> Html {
    html! {
        <svg class="icon" viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true">
            <path d="M12 .3a12 12 0 0 0-3.8 23.4c.6.1.8-.3.8-.6v-2c-3.3.7-4-1.6-4-1.6-.6-1.4-1.4-1.8-1.4-1.8-1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1.1 1.8 2.8 1.3 3.5 1 .1-.8.4-1.3.8-1.6-2.7-.3-5.5-1.3-5.5-5.9 0-1.3.5-2.4 1.2-3.2-.1-.3-.5-1.5.1-3.2 0 0 1-.3 3.3 1.2a11.5 11.5 0 0 1 6 0C17.3 4.7 18.3 5 18.3 5c.6 1.7.2 2.9.1 3.2.8.8 1.2 1.9 1.2 3.2 0 4.6-2.8 5.6-5.5 5.9.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6A12 12 0 0 0 12 .3" />
        </svg>
    }
}

pub fn copy() -> Html {
    stroked(html! { <path d="M9 9h11v11H9zM5 15H4V4h11v1" /> })
}

pub fn check() -> Html {
    stroked(html! { <path d="M20 6L9 17l-5-5" /> })
}

/// The icons the front page's content refers to by name.
pub fn named(name: &str) -> Html {
    match name {
        "code" => stroked(html! { <path d="M16 18l6-6-6-6M8 6l-6 6 6 6" /> }),
        "async" => stroked(html! { <path d="M16 3h5v5M4 20L21 3M21 16v5h-5M15 15l6 6M4 4l5 5" /> }),
        "plug" => stroked(html! {
            <path d="M21 16V8l-9-5-9 5v8l9 5zM3.3 7L12 12l8.7-5M12 22V12" />
        }),
        "shield" => stroked(html! { <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" /> }),
        "zap" => stroked(html! { <path d="M13 2L3 14h9l-1 8 10-12h-9z" /> }),
        "tool" => stroked(html! {
            <path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.8-3.8a6 6 0 0 1-7.9 7.9l-6.9 6.9a2.1 2.1 0 0 1-3-3l6.9-6.9a6 6 0 0 1 7.9-7.9z" />
        }),
        _ => Html::default(),
    }
}
