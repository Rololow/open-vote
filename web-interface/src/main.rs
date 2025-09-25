mod components;
mod services;
mod types;

use components::Dashboard;

use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! {
        <div class="app">
            <header class="header">
                <h1>{"🔗 Analyseur Blockchain E-Government"}</h1>
                <p>{"Tableau de bord de surveillance et d'analyse en temps réel"}</p>
            </header>
            <main>
                <Dashboard />
            </main>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}