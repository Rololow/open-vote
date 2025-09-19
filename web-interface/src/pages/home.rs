use yew::prelude::*;
use crate::pages::auth::UserAccount;

#[derive(Properties, PartialEq)]
pub struct HomeProps {
    pub user: Option<UserAccount>,
    pub on_navigate: Callback<String>,
}

#[function_component(HomePage)]
pub fn home_page(props: &HomeProps) -> Html {
    let user = &props.user;
    let on_navigate = &props.on_navigate;

    html! {
        <div class="home-container">
            <div class="hero-section">
                <h1>{"🏛️ E-Government Blockchain"}</h1>
                <p class="hero-subtitle">{"Système de gouvernance décentralisée et transparente"}</p>
                
                if let Some(user) = user {
                    <div class="welcome-card">
                        <h3>{format!("Bienvenue, {} !", user.username)}</h3>
                        <div class="user-stats">
                            <div class="stat">
                                <span class="stat-label">{"Réputation:"}</span>
                                <span class="stat-value">{user.reputation}</span>
                            </div>
                            <div class="stat">
                                <span class="stat-label">{"ID:"}</span>
                                <span class="stat-value">{user.id.to_string()[..8].to_string()}{"..."}</span>
                            </div>
                        </div>
                    </div>
                } else {
                    <div class="cta-section">
                        <p>{"Participez à la démocratie numérique"}</p>
                        <button 
                            class="btn-primary"
                            onclick={
                                let on_navigate = on_navigate.clone();
                                Callback::from(move |_| on_navigate.emit("register".to_string()))
                            }
                        >
                            {"Créer un compte"}
                        </button>
                    </div>
                }
            </div>

            <div class="features-section">
                <h2>{"Fonctionnalités principales"}</h2>
                <div class="features-grid">
                    <div class="feature-card" onclick={
                        let on_navigate = on_navigate.clone();
                        Callback::from(move |_| on_navigate.emit("laws".to_string()))
                    }>
                        <div class="feature-icon">{"📜"}</div>
                        <h3>{"Consultation des lois"}</h3>
                        <p>{"Parcourez et consultez toutes les lois en vigueur et en discussion"}</p>
                        <div class="feature-status">{"Disponible"}</div>
                    </div>

                    <div class="feature-card" onclick={
                        let on_navigate = on_navigate.clone();
                        Callback::from(move |_| on_navigate.emit("proposals".to_string()))
                    }>
                        <div class="feature-icon">{"✏️"}</div>
                        <h3>{"Propositions de modifications"}</h3>
                        <p>{"Proposez des amendements et de nouvelles lois pour améliorer la société"}</p>
                        <div class="feature-status">{"Disponible"}</div>
                    </div>

                    <div class="feature-card" onclick={
                        let on_navigate = on_navigate.clone();
                        Callback::from(move |_| on_navigate.emit("voting".to_string()))
                    }>
                        <div class="feature-icon">{"🗳️"}</div>
                        <h3>{"Système de vote"}</h3>
                        <p>{"Votez sur les propositions et participez aux décisions démocratiques"}</p>
                        <div class="feature-status">{"Disponible"}</div>
                    </div>

                    <div class="feature-card">
                        <div class="feature-icon">{"🔗"}</div>
                        <h3>{"Blockchain Explorer"}</h3>
                        <p>{"Explorez la blockchain et vérifiez la transparence des votes"}</p>
                        <div class="feature-status coming-soon">{"Bientôt disponible"}</div>
                    </div>

                    <div class="feature-card">
                        <div class="feature-icon">{"📊"}</div>
                        <h3>{"Statistiques et analyses"}</h3>
                        <p>{"Consultez les statistiques de participation et l'évolution des votes"}</p>
                        <div class="feature-status coming-soon">{"Bientôt disponible"}</div>
                    </div>

                    <div class="feature-card">
                        <div class="feature-icon">{"🔔"}</div>
                        <h3>{"Notifications"}</h3>
                        <p>{"Recevez des alertes sur les nouvelles propositions et votes importants"}</p>
                        <div class="feature-status coming-soon">{"Bientôt disponible"}</div>
                    </div>
                </div>
            </div>

            <div class="stats-section">
                <h2>{"État du système"}</h2>
                <div class="stats-grid">
                    <div class="stat-card">
                        <div class="stat-number">{"23"}</div>
                        <div class="stat-label">{"Lois actives"}</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-number">{"147"}</div>
                        <div class="stat-label">{"Citoyens inscrits"}</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-number">{"8"}</div>
                        <div class="stat-label">{"Votes en cours"}</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-number">{"1,234"}</div>
                        <div class="stat-label">{"Transactions blockchain"}</div>
                    </div>
                </div>
            </div>

            <div class="how-it-works">
                <h2>{"Comment ça fonctionne"}</h2>
                <div class="steps">
                    <div class="step">
                        <div class="step-number">{"1"}</div>
                        <div class="step-content">
                            <h4>{"Créez votre compte"}</h4>
                            <p>{"Inscrivez-vous avec une paire de clés cryptographiques unique"}</p>
                        </div>
                    </div>
                    <div class="step">
                        <div class="step-number">{"2"}</div>
                        <div class="step-content">
                            <h4>{"Explorez les lois"}</h4>
                            <p>{"Consultez les textes existants et les propositions en discussion"}</p>
                        </div>
                    </div>
                    <div class="step">
                        <div class="step-number">{"3"}</div>
                        <div class="step-content">
                            <h4>{"Proposez et votez"}</h4>
                            <p>{"Soumettez vos idées et participez aux votes démocratiques"}</p>
                        </div>
                    </div>
                    <div class="step">
                        <div class="step-number">{"4"}</div>
                        <div class="step-content">
                            <h4>{"Transparent et immuable"}</h4>
                            <p>{"Toutes les actions sont enregistrées sur la blockchain"}</p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}