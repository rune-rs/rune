//! The site, as a single page application over the content `tools/site`
//! writes to `/content`.

use std::rc::Rc;

use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlAnchorElement, MouseEvent};
use yew::prelude::*;
use yew_router::prelude::*;

use crate::editor::{self, Editor};
use crate::icons;
use crate::model::{self, Block, Document, PostSummary, Site};
use crate::theme::ThemeToggle;

#[derive(Debug, Clone, PartialEq, Routable)]
enum Route {
    #[at("/")]
    Home,
    #[at("/posts/")]
    Posts,
    #[at("/posts/:slug/")]
    Post { slug: String },
    #[at("/play/")]
    Playground,
    #[at("/:name/")]
    Page { name: String },
    #[not_found]
    #[at("/404/")]
    NotFound,
}

#[wasm_bindgen(start)]
pub fn start() {
    let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("app"))
    else {
        return;
    };

    // What the page shows until the app has loaded.
    root.set_inner_html("");
    yew::Renderer::<App>::with_root(root).render();
}

#[derive(Clone, PartialEq)]
enum Load<T> {
    Loading,
    Ready(T),
    Missing,
}

async fn fetch<T: DeserializeOwned>(url: &str) -> Option<T> {
    let response = Request::get(url).send().await.ok()?;

    if !response.ok() {
        return None;
    }

    response.json().await.ok()
}

#[hook]
fn use_content<T>(url: String) -> Load<Rc<T>>
where
    T: DeserializeOwned + 'static,
{
    let state = use_state(|| (String::new(), Load::Loading));

    {
        let state = state.clone();

        use_effect_with(url.clone(), move |url| {
            let url = url.clone();
            state.set((url.clone(), Load::Loading));

            spawn_local(async move {
                let load = match fetch::<T>(&url).await {
                    Some(value) => Load::Ready(Rc::new(value)),
                    None => Load::Missing,
                };

                state.set((url, load));
            });
        });
    }

    // A response for the previous page is not this page's.
    match &*state {
        (current, load) if *current == url => load.clone(),
        _ => Load::Loading,
    }
}

#[hook]
fn use_title(title: Option<String>) {
    let site = use_context::<Rc<Site>>();

    use_effect_with((title, site), |(title, site)| {
        let site = site.as_ref().map(|s| s.title.as_str()).unwrap_or("Rune");

        let title = match title {
            Some(title) => format!("{title} | {site}"),
            None => site.to_owned(),
        };

        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            document.set_title(&title);
        }
    });
}

#[function_component]
fn App() -> Html {
    let site = use_content::<Site>(String::from("/content/site.json"));

    match site {
        Load::Loading => html! { <div class="boot" aria-busy="true"></div> },
        Load::Missing => html! {
            <div class="boot">
                <p class="muted">{"The site's content could not be loaded."}</p>
            </div>
        },
        Load::Ready(site) => html! {
            <ContextProvider<Rc<Site>> context={site}>
                <BrowserRouter>
                    <Links />
                    <Shell>
                        <Switch<Route> render={switch} />
                    </Shell>
                </BrowserRouter>
            </ContextProvider<Rc<Site>>>
        },
    }
}

fn switch(route: Route) -> Html {
    match route {
        Route::Home => html! { <Home /> },
        Route::Posts => html! { <Posts /> },
        Route::Post { slug } => html! { <Post {slug} /> },
        Route::Playground => html! { <Playground /> },
        Route::Page { name } => html! { <Page {name} /> },
        Route::NotFound => html! { <NotFound /> },
    }
}

/// Follows links to the app's own pages without reloading it, since a reload
/// loads the whole compiler again. Markdown content links with plain anchors,
/// so this is done for the document rather than per link.
#[function_component]
fn Links() -> Html {
    let navigator = use_navigator();
    let location = use_location();
    let site = use_context::<Rc<Site>>();

    use_effect_with((navigator, site), |(navigator, site)| {
        let navigator = navigator.clone();
        let site = site.clone();

        let listener = Closure::<dyn Fn(MouseEvent)>::new(move |event: MouseEvent| {
            if event.default_prevented()
                || event.button() != 0
                || event.ctrl_key()
                || event.meta_key()
                || event.shift_key()
                || event.alt_key()
            {
                return;
            }

            let Some(anchor) = event
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .and_then(|e| e.closest("a").ok().flatten())
                .and_then(|e| e.dyn_into::<HtmlAnchorElement>().ok())
            else {
                return;
            };

            let Some(origin) = web_sys::window().and_then(|w| w.location().origin().ok()) else {
                return;
            };

            if !anchor.target().is_empty()
                || anchor.has_attribute("download")
                || !anchor.href().starts_with(&origin)
            {
                return;
            }

            let path = anchor.pathname();

            let known = match Route::recognize(&path) {
                Some(Route::NotFound) | None => false,
                Some(Route::Page { name }) => {
                    site.as_ref().is_some_and(|s| s.pages.contains(&name))
                }
                Some(_) => true,
            };

            if let (true, Some(navigator)) = (known, &navigator) {
                if let Some(route) = Route::recognize(&path) {
                    event.prevent_default();
                    navigator.push(&route);
                }
            }
        });

        let document = web_sys::window().and_then(|w| w.document());

        if let Some(document) = &document {
            let _ = document
                .add_event_listener_with_callback("click", listener.as_ref().unchecked_ref());
        }

        move || {
            if let Some(document) = &document {
                let _ = document.remove_event_listener_with_callback(
                    "click",
                    listener.as_ref().unchecked_ref(),
                );
            }
        }
    });

    use_effect_with(location.map(|l| l.path().to_owned()), |_| {
        if let Some(window) = web_sys::window() {
            window.scroll_to_with_x_and_y(0.0, 0.0);
        }
    });

    Html::default()
}

#[derive(Properties, PartialEq)]
struct ShellProps {
    children: Html,
}

#[function_component]
fn Shell(props: &ShellProps) -> Html {
    let site = use_context::<Rc<Site>>().expect("site context");
    let route = use_route::<Route>();
    let links = &site.links;

    let wide = matches!(route, Some(Route::Playground | Route::Home));
    // The playground is the window, with nothing below it to scroll to.
    let fill = matches!(route, Some(Route::Playground));

    let nav = |target: Route, label: &'static str| {
        let current = match (&route, &target) {
            (Some(Route::Post { .. }), Route::Posts) => true,
            (Some(route), target) => route == target,
            _ => false,
        };

        html! {
            <Link<Route> to={target} classes={classes!("nav-link", current.then_some("active"))}>
                {label}
            </Link<Route>>
        }
    };

    html! {
        <div class={classes!("shell", fill.then_some("fill"))}>
        <a class="skip-link" href="#main">{"Skip to content"}</a>
        <header class="site-header">
            <div class="site-header-inner">
                <Link<Route> to={Route::Home} classes="brand">
                    <img class="brand-logo" src="/img/logo-256.png" alt="" width="28" height="28" />
                    <span class="brand-name">{"Rune"}</span>
                </Link<Route>>
                <nav class="nav" aria-label="Main">
                    {nav(Route::Posts, "Blog")}
                    {nav(Route::Playground, "Playground")}
                    <a class="nav-link" href={links.book.clone()}>{"Book"}</a>
                    <a class="nav-link" href={links.docs.clone()}>{"Docs"}</a>
                </nav>
                <div class="cluster tight header-actions">
                    <a class="icon-button" href={links.github.clone()} title="GitHub" aria-label="GitHub">{icons::github()}</a>
                    <a class="icon-button" href={links.discord.clone()} title="Discord" aria-label="Discord">{icons::chat()}</a>
                    <ThemeToggle />
                </div>
            </div>
        </header>
        <main id="main" class={classes!("container", wide.then_some("wide"), fill.then_some("fill"))}>
            {props.children.clone()}
        </main>
        if !fill {
        <footer class="site-footer">
            <div class="container wide cluster spread">
                <nav class="cluster" aria-label="Elsewhere">
                    <a class="footer-link" href={links.book.clone()}>{"The book"}</a>
                    <a class="footer-link" href={links.docs.clone()}>{"Rune docs"}</a>
                    <a class="footer-link" href={links.api.clone()}>{"API docs"}</a>
                    <a class="footer-link" href={links.github.clone()}>{"GitHub"}</a>
                    <a class="footer-link" href={links.discord.clone()}>{"Discord"}</a>
                    <a class="footer-link" href="/rss.xml">{"RSS"}</a>
                </nav>
                <p class="footer-note muted small">{"Rune is dual licensed under MIT or Apache-2.0."}</p>
            </div>
        </footer>
        }
        </div>
    }
}

fn blocks(key: &str, document: &Document) -> Html {
    let blocks = document.blocks.iter().enumerate().map(|(index, block)| {
        // Keyed by page, so that an editor is not reused for another page's
        // snippet: it only reads its source when it is created.
        let key = format!("{key}/{index}");

        match block {
            Block::Html { html } => html! {
                <div {key} class="prose-part">{Html::from_html_unchecked(html.clone().into())}</div>
            },
            Block::Editor { props, caption } => html! {
                <figure {key} class="snippet">
                    <Editor ..props.clone() />
                    if let Some(caption) = caption {
                        <figcaption class="snippet-caption">{caption}</figcaption>
                    }
                </figure>
            },
        }
    });

    html! { <div class="prose">{for blocks}</div> }
}

fn loading() -> Html {
    html! { <div class="loading" aria-busy="true"></div> }
}

#[function_component]
fn Home() -> Html {
    let site = use_context::<Rc<Site>>().expect("site context");
    let home = use_content::<model::Home>(String::from("/content/home.json"));
    use_title(None);

    let home = match &home {
        Load::Loading => return loading(),
        Load::Missing => return missing(),
        Load::Ready(home) => home,
    };

    let hero = &home.hero;
    let links = &site.links;

    let features = home.features.iter().map(|feature| {
        html! {
            <article class="card feature">
                <span class="feature-icon">{icons::named(&feature.icon)}</span>
                <h3 class="feature-title">{&feature.title}</h3>
                <p class="feature-text">{raw(&feature.text)}</p>
            </article>
        }
    });

    html! {
        <>
        <section class="hero">
            <div class="hero-pitch">
                <p class="eyebrow">{&hero.eyebrow}</p>
                <h1 class="hero-title">{&hero.headline}</h1>
                <p class="hero-lead">{raw(&hero.lead)}</p>
                <div class="cluster">
                    <Link<Route> to={Route::Playground} classes="button primary large">{icons::play()}{"Try it now"}</Link<Route>>
                    <a class="button large" href={links.book.clone()}>{icons::book()}{"Read the book"}</a>
                </div>
                <Install command={hero.install.clone()} />
            </div>
            <div class="window hero-demo">
                <div class="window-bar">
                    <span class="window-dots" aria-hidden="true"><span /><span /><span /></span>
                    <span class="window-title">{&hero.file}</span>
                    <span class="window-hint">{"Edit me"}</span>
                </div>
                <Editor ..hero.editor.clone() />
            </div>
        </section>

        <section class="section">
            <header class="section-head">
                <p class="eyebrow">{"Why Rune"}</p>
                <h2 class="section-title">{"Everything you want from a scripting language. Nothing you have to fight."}</h2>
            </header>
            <div class="grid">{for features}</div>
        </section>

        <section class="section">
            <header class="section-head">
                <p class="eyebrow">{"Live examples"}</p>
                <h2 class="section-title">{"Don't take our word for it. Run it."}</h2>
                <p class="section-lead">{"Every example below compiles and runs right here in your browser, on the same virtual machine you would embed. Change anything you like."}</p>
            </header>
            <Examples examples={home.examples.clone()} />
        </section>

        {showcase(&home.safeguards, "budget.rn")}
        {showcase(&home.embed, "main.rs")}

        <section class="closing">
            <h2 class="section-title">{&home.closing.title}</h2>
            <p class="section-lead">{raw(&home.closing.text)}</p>
            <div class="cluster center">
                <Link<Route> to={Route::Playground} classes="button primary large">{icons::play()}{"Open the playground"}</Link<Route>>
                <a class="button large" href={links.book.clone()}>{icons::book()}{"Read the book"}</a>
                <a class="button large" href={links.github.clone()}>{icons::github()}{"GitHub"}</a>
                <a class="button large" href={links.discord.clone()}>{icons::chat()}{"Discord"}</a>
            </div>
        </section>
        </>
    }
}

fn raw(html: &str) -> Html {
    Html::from_html_unchecked(html.to_owned().into())
}

fn showcase(showcase: &model::Showcase, file: &str) -> Html {
    let demo = match (&showcase.editor, &showcase.code) {
        (Some(editor), _) => html! {
            <div class="window">
                <div class="window-bar">
                    <span class="window-dots" aria-hidden="true"><span /><span /><span /></span>
                    <span class="window-title">{file}</span>
                </div>
                <Editor ..editor.clone() />
            </div>
        },
        (None, Some(code)) => html! {
            <div class="window">
                <div class="window-bar">
                    <span class="window-dots" aria-hidden="true"><span /><span /><span /></span>
                    <span class="window-title">{file}</span>
                </div>
                {raw(code)}
            </div>
        },
        (None, None) => Html::default(),
    };

    let points = showcase.points.iter().map(|point| {
        html! {
            <li class="checklist-item">
                <span class="checklist-mark">{icons::check()}</span>
                <span>
                    <strong class="checklist-title">{&point.title}</strong>
                    {" "}{raw(&point.text)}
                </span>
            </li>
        }
    });

    html! {
        <section class="section split">
            <div class="split-text">
                <p class="eyebrow">{&showcase.eyebrow}</p>
                <h2 class="section-title">{&showcase.title}</h2>
                <p class="section-lead">{raw(&showcase.text)}</p>
                if !showcase.points.is_empty() {
                    <ul class="checklist">{for points}</ul>
                }
                if let Some(note) = &showcase.note {
                    <p class="callout">
                        <strong class="callout-title">{"Where this is going. "}</strong>
                        {raw(note)}
                    </p>
                }
            </div>
            {demo}
        </section>
    }
}

#[derive(Properties, PartialEq)]
struct ExamplesProps {
    examples: Vec<model::Example>,
}

#[function_component]
fn Examples(props: &ExamplesProps) -> Html {
    let selected = use_state(|| 0usize);

    let tabs = props.examples.iter().enumerate().map(|(index, example)| {
        let onclick = {
            let selected = selected.clone();
            Callback::from(move |_| selected.set(index))
        };

        let current = *selected == index;

        html! {
            <button type="button" role="tab" class={classes!("tab", current.then_some("active"))}
                aria-selected={current.to_string()} {onclick}>
                {&example.title}
            </button>
        }
    });

    let Some(example) = props.examples.get(*selected) else {
        return Html::default();
    };

    html! {
        <div class="examples">
            <div class="tabs" role="tablist" aria-label="Examples">{for tabs}</div>
            <p class="examples-text">{raw(&example.text)}</p>
            // Keyed, so that each example gets a fresh editor: an editor reads
            // its source only when it is created.
            <div class="window" key={*selected}>
                <Editor ..example.editor.clone() />
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct InstallProps {
    command: String,
}

#[function_component]
fn Install(props: &InstallProps) -> Html {
    let copied = use_state(|| false);

    let onclick = {
        let copied = copied.clone();
        let command = props.command.clone();

        Callback::from(move |_| {
            let Some(window) = web_sys::window() else {
                return;
            };

            let _ = window.navigator().clipboard().write_text(&command);
            copied.set(true);
        })
    };

    let label = if *copied {
        "Copied"
    } else {
        "Copy to clipboard"
    };

    html! {
        <div class="install">
            <span class="install-prompt" aria-hidden="true">{"$"}</span>
            <code class="install-command">{&props.command}</code>
            <button type="button" class="icon-button small" title={label} aria-label={label} {onclick}>
                {if *copied { icons::check() } else { icons::copy() }}
            </button>
        </div>
    }
}

#[function_component]
fn Posts() -> Html {
    let site = use_context::<Rc<Site>>().expect("site context");
    use_title(Some(String::from("Blog")));

    let posts = site.posts.iter().map(|post| {
        html! {
            <article class="card post-card">
                {meta(post)}
                <h2 class="post-card-title">
                    <Link<Route> to={Route::Post { slug: post.slug.clone() }} classes="post-card-link">{&post.title}</Link<Route>>
                </h2>
                <div class="prose">{Html::from_html_unchecked(post.summary.clone().into())}</div>
                <Link<Route> to={Route::Post { slug: post.slug.clone() }} classes="read-more">
                    {"Read more"}{icons::arrow()}
                </Link<Route>>
            </article>
        }
    });

    html! {
        <>
        <header class="page-head split">
            <div class="page-head-text">
                <h1 class="page-title">{"Blog"}</h1>
                <p class="page-lead">{"News and notes from the development of Rune."}</p>
            </div>
            <a class="button small" href="/rss.xml">{icons::rss()}{"RSS feed"}</a>
        </header>
        <div class="stack">{for posts}</div>
        </>
    }
}

fn meta(post: &PostSummary) -> Html {
    html! {
        <p class="meta">
            if let Some(author) = &post.author {
                <span>{author}</span>
                <span aria-hidden="true">{"·"}</span>
            }
            <time datetime={post.date.clone()}>{&post.date_long}</time>
            <span aria-hidden="true">{"·"}</span>
            <span>{format!("{} min read", post.minutes)}</span>
        </p>
    }
}

#[derive(Properties, PartialEq)]
struct PostProps {
    slug: String,
}

#[function_component]
fn Post(props: &PostProps) -> Html {
    let site = use_context::<Rc<Site>>().expect("site context");
    let document = use_content::<Document>(format!("/content/posts/{}.json", props.slug));
    let summary = site.posts.iter().find(|p| p.slug == props.slug);
    use_title(summary.map(|s| s.title.clone()));

    let Some(summary) = summary else {
        return html! { <NotFound /> };
    };

    let body = match &document {
        Load::Loading => loading(),
        Load::Ready(document) => blocks(&props.slug, document),
        Load::Missing => missing(),
    };

    html! {
        <article class="article">
            <header class="page-head">
                <Link<Route> to={Route::Posts} classes="back-link">{"← Blog"}</Link<Route>>
                <h1 class="page-title">{&summary.title}</h1>
                {meta(summary)}
            </header>
            {body}
        </article>
    }
}

#[function_component]
fn Playground() -> Html {
    let site = use_context::<Rc<Site>>().expect("site context");
    use_title(Some(String::from("Playground")));

    let props = editor::Props {
        source: site.playground.trim().to_owned(),
        share: true,
        storage_key: Some(String::from("playground")),
        run_on_change: true,
        ..editor::Props::default()
    };

    html! {
        <>
        <header class="page-head compact">
            <h1 class="page-title small">{"Playground"}</h1>
            <p class="page-lead small">
                {"Write Rune and run it in your browser. "}
                <kbd>{"Ctrl"}</kbd>{" + "}<kbd>{"Enter"}</kbd>
                {" runs it, your code is kept in this browser between visits, and Share copies a link to it."}
            </p>
        </header>
        <div class="playground">
            <Editor ..props />
        </div>
        </>
    }
}

#[derive(Properties, PartialEq)]
struct PageProps {
    name: String,
}

#[function_component]
fn Page(props: &PageProps) -> Html {
    let document = use_content::<Document>(format!("/content/pages/{}.json", props.name));
    let title = match &document {
        Load::Ready(document) => document.title.clone(),
        _ => None,
    };

    use_title(title);

    match &document {
        Load::Loading => loading(),
        Load::Ready(document) => blocks(&props.name, document),
        Load::Missing => html! { <NotFound /> },
    }
}

fn missing() -> Html {
    html! { <p class="muted">{"This page's content could not be loaded."}</p> }
}

#[function_component]
fn NotFound() -> Html {
    use_title(Some(String::from("Not found")));

    html! {
        <section class="empty">
            <h1 class="page-title">{"Nothing here"}</h1>
            <p class="page-lead">{"The page you were looking for does not exist, or has moved."}</p>
            <Link<Route> to={Route::Home} classes="button">{"Go to the front page"}</Link<Route>>
        </section>
    }
}
