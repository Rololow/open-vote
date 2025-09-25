use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct StatsCardProps {
    pub title: String,
    pub value: String,
    pub subtitle: String,
}

#[function_component(StatsCard)]
pub fn stats_card(props: &StatsCardProps) -> Html {
    html! {
        <div class="card">
            <h3>{&props.title}</h3>
            <div class="stat-value">{&props.value}</div>
            <div class="stat-label">{&props.subtitle}</div>
        </div>
    }
}