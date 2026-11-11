use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes, A},
    hooks::use_params_map,
    ParamSegment, StaticSegment,
};

#[derive(Clone, Copy, Debug)]
pub struct UserId(pub u64);

impl std::str::FromStr for UserId {
    type Err = ();

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(());
        }
        raw.parse().map(Self).map_err(|_| ())
    }
}

pub fn is_username(raw: &str) -> bool {
    let mut bytes = raw.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[server(name = EchoMessage, prefix = "/api", endpoint = "echo-message")]
pub async fn echo_message(message: String) -> Result<String, ServerFnError> {
    if message == "reject" {
        return Err(ServerFnError::new("synthetic form failure"));
    }
    Ok(format!("Echo: {message}"))
}

#[server(name = LateText, prefix = "/api", endpoint = "late-text")]
pub async fn late_text() -> Result<String, ServerFnError> {
    let gate = expect_context::<std::sync::Arc<tokio::sync::Notify>>();
    gate.notified().await;
    Ok("Late content arrived".into())
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body><App/></body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let hydrated = RwSignal::new(false);
    Effect::new(move |_| hydrated.set(true));
    view! {
        <Stylesheet id="leptos" href="/pkg/warded-leptos-probe.css"/>
        <Title text="Welcome to Leptos"/>
        <Router>
            <nav>
                <A href="/">"Home"</A>" "
                <A href="/users/42">"Numeric user"</A>" "
                <A href="/users/alice">"Named user"</A>" "
                <A href="/form">"Form"</A>" "
                <A href="/stream">"Stream"</A>
            </nav>
            <main data-hydrated=move || hydrated.get().to_string()>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                    <Route path=(StaticSegment("users"), ParamSegment("value")) view=UserPage/>
                    <Route path=StaticSegment("form") view=FormPage/>
                    <Route path=StaticSegment("stream") view=StreamPage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let count = RwSignal::new(0);
    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=move |_| *count.write() += 1>"Click Me: " {count}</button>
    }
}

#[component]
fn UserPage() -> impl IntoView {
    let params = use_params_map();
    view! {
        <h1>"User"</h1>
        <p data-testid="user-value">{move || params.read().get("value").map(|v| v.to_owned()).unwrap_or_default()}</p>
    }
}

#[component]
fn FormPage() -> impl IntoView {
    let action = ServerAction::<EchoMessage>::new();
    view! {
        <h1>"Echo form"</h1>
        <ActionForm action=action>
            <label for="message">"Message"</label>
            <input id="message" name="message"/>
            <button type="submit" disabled=move || action.pending().get()>"Send message"</button>
        </ActionForm>
        <output data-testid="form-result">{move || action.value().get().map(|result| match result {
            Ok(value) => value,
            Err(error) => format!("Error: {error}"),
        })}</output>
    }
}

#[component]
fn StreamPage() -> impl IntoView {
    let late = Resource::new(|| (), |_| late_text());
    view! {
        <h1 data-testid="stream-start">"Stream started"</h1>
        <Suspense fallback=|| view! { <p data-testid="stream-waiting">"Waiting for release"</p> }>
            {move || Suspend::new(async move {
                let text = late.await.unwrap_or_else(|error| format!("Error: {error}"));
                view! { <p data-testid="stream-late">{text}</p> }
            })}
        </Suspense>
    }
}
