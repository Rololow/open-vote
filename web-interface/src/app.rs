use yew::prelude::*;
use pages::{HomePage, LoginPage, RegisterPage, LawsPage, ProposalsPage, VotingPage, ProfilePage};
use pages::auth::UserAccount;
use gloo::storage::{LocalStorage, Storage};

mod pages;

#[derive(Clone, PartialEq)]
pub enum Route {
    Home,
    Login,
    Register,
    Laws,
    Proposals,
    Voting,
    Profile,
}

#[function_component(App)]
pub fn app() -> Html {
    let current_route = use_state(|| Route::Home);
    let current_user = use_state(|| -> Option<UserAccount> {
        match LocalStorage::get("current_user") {
            Ok(user) => Some(user),
            Err(_) => None,
        }
    });

    // Callbacks pour la navigation
    let on_navigate = {
        let current_route = current_route.clone();
        Callback::from(move |route: String| {
            let new_route = match route.as_str() {
                "home" => Route::Home,
                "login" => Route::Login,
                "register" => Route::Register,
                "laws" => Route::Laws,
                "proposals" => Route::Proposals,
                "voting" => Route::Voting,
                "profile" => Route::Profile,
                _ => Route::Home,
            };
            current_route.set(new_route);
        })
    };

    let on_login = {
        let current_user = current_user.clone();
        let current_route = current_route.clone();
        Callback::from(move |user: UserAccount| {
            current_user.set(Some(user));
            current_route.set(Route::Home);
        })
    };

    let on_logout = {
        let current_user = current_user.clone();
        let current_route = current_route.clone();
        Callback::from(move |_| {
            current_user.set(None);
            current_route.set(Route::Home);
        })
    };

    let render_navigation = {
        let current_route = current_route.clone();
        let current_user = current_user.clone();
        let on_navigate = on_navigate.clone();
        let on_logout = on_logout.clone();

        move || {
            html! {
                <nav class="app-nav">
                    <div class="nav-container">
                        <a 
                            href="#" 
                            class="nav-brand"
                            onclick={
                                let on_navigate = on_navigate.clone();
                                Callback::from(move |e: MouseEvent| {
                                    e.prevent_default();
                                    on_navigate.emit("home".to_string());
                                })
                            }
                        >
                            {"🏛️ E-Government"}
                        </a>
                        
                        <div class="nav-menu">
                            <ul class="nav-links">
                                <li>
                                    <a 
                                        href="#" 
                                        class={if *current_route == Route::Home { "nav-link active" } else { "nav-link" }}
                                        onclick={
                                            let on_navigate = on_navigate.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.prevent_default();
                                                on_navigate.emit("home".to_string());
                                            })
                                        }
                                    >
                                        {"🏠 Accueil"}
                                    </a>
                                </li>
                                <li>
                                    <a 
                                        href="#" 
                                        class={if *current_route == Route::Laws { "nav-link active" } else { "nav-link" }}
                                        onclick={
                                            let on_navigate = on_navigate.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.prevent_default();
                                                on_navigate.emit("laws".to_string());
                                            })
                                        }
                                    >
                                        {"📜 Lois"}
                                    </a>
                                </li>
                                <li>
                                    <a 
                                        href="#" 
                                        class={if *current_route == Route::Proposals { "nav-link active" } else { "nav-link" }}
                                        onclick={
                                            let on_navigate = on_navigate.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.prevent_default();
                                                on_navigate.emit("proposals".to_string());
                                            })
                                        }
                                    >
                                        {"✏️ Propositions"}
                                    </a>
                                </li>
                                <li>
                                    <a 
                                        href="#" 
                                        class={if *current_route == Route::Voting { "nav-link active" } else { "nav-link" }}
                                        onclick={
                                            let on_navigate = on_navigate.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.prevent_default();
                                                on_navigate.emit("voting".to_string());
                                            })
                                        }
                                    >
                                        {"🗳️ Votes"}
                                    </a>
                                </li>
                            </ul>
                            
                            <div class="nav-user">
                                if let Some(user) = (*current_user).as_ref() {
                                    <div class="user-info">
                                        <div class="user-avatar">
                                            {user.username.chars().next().unwrap_or('?').to_uppercase().to_string()}
                                        </div>
                                        <span>{&user.username}</span>
                                    </div>
                                    <a 
                                        href="#" 
                                        class={if *current_route == Route::Profile { "nav-link active" } else { "nav-link" }}
                                        onclick={
                                            let on_navigate = on_navigate.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.prevent_default();
                                                on_navigate.emit("profile".to_string());
                                            })
                                        }
                                    >
                                        {"👤 Profil"}
                                    </a>
                                    <button 
                                        class="btn btn-secondary btn-sm"
                                        onclick={
                                            let on_logout = on_logout.clone();
                                            Callback::from(move |_| on_logout.emit(()))
                                        }
                                    >
                                        {"🚪 Déconnexion"}
                                    </button>
                                } else {
                                    <a 
                                        href="#" 
                                        class={if *current_route == Route::Login { "nav-link active" } else { "nav-link" }}
                                        onclick={
                                            let on_navigate = on_navigate.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.prevent_default();
                                                on_navigate.emit("login".to_string());
                                            })
                                        }
                                    >
                                        {"🔐 Connexion"}
                                    </a>
                                    <a 
                                        href="#" 
                                        class={if *current_route == Route::Register { "nav-link active" } else { "nav-link" }}
                                        onclick={
                                            let on_navigate = on_navigate.clone();
                                            Callback::from(move |e: MouseEvent| {
                                                e.prevent_default();
                                                on_navigate.emit("register".to_string());
                                            })
                                        }
                                    >
                                        {"📝 Inscription"}
                                    </a>
                                }
                            </div>
                        </div>
                    </div>
                </nav>
            }
        }
    };

    let render_content = {
        let current_route = current_route.clone();
        let current_user = current_user.clone();
        let on_navigate = on_navigate.clone();
        let on_login = on_login.clone();
        let on_logout = on_logout.clone();

        move || {
            match *current_route {
                Route::Home => html! {
                    <HomePage 
                        user={(*current_user).clone()} 
                        on_navigate={on_navigate.clone()} 
                    />
                },
                Route::Login => html! {
                    <LoginPage 
                        on_login={on_login.clone()} 
                        on_navigate={on_navigate.clone()} 
                    />
                },
                Route::Register => html! {
                    <RegisterPage 
                        on_register={on_login.clone()} 
                        on_navigate={on_navigate.clone()} 
                    />
                },
                Route::Laws => html! {
                    <LawsPage 
                        user={(*current_user).clone()} 
                        on_navigate={on_navigate.clone()} 
                    />
                },
                Route::Proposals => html! {
                    <ProposalsPage 
                        user={(*current_user).clone()} 
                        on_navigate={on_navigate.clone()} 
                    />
                },
                Route::Voting => html! {
                    <VotingPage 
                        user={(*current_user).clone()} 
                        on_navigate={on_navigate.clone()} 
                    />
                },
                Route::Profile => html! {
                    <ProfilePage 
                        user={(*current_user).clone()} 
                        on_logout={on_logout.clone()} 
                        on_navigate={on_navigate.clone()} 
                    />
                },
            }
        }
    };

    html! {
        <div class="app-container">
            {render_navigation()}
            <main class="main-content">
                {render_content()}
            </main>
            <footer class="app-footer">
                <div class="footer-content">
                    <div class="footer-section">
                        <h3>{"🏛️ E-Government Blockchain"}</h3>
                        <p>{"Système de gouvernance décentralisée utilisant la technologie blockchain pour assurer la transparence et la participation démocratique."}</p>
                    </div>
                    <div class="footer-section">
                        <h3>{"🔗 Liens rapides"}</h3>
                        <ul>
                            <li><a href="#">{"Documentation"}</a></li>
                            <li><a href="#">{"Code source"}</a></li>
                            <li><a href="#">{"Support"}</a></li>
                            <li><a href="#">{"À propos"}</a></li>
                        </ul>
                    </div>
                    <div class="footer-section">
                        <h3>{"🛡️ Sécurité"}</h3>
                        <ul>
                            <li><a href="#">{"Politique de confidentialité"}</a></li>
                            <li><a href="#">{"Conditions d'utilisation"}</a></li>
                            <li><a href="#">{"Audit de sécurité"}</a></li>
                            <li><a href="#">{"Signaler un problème"}</a></li>
                        </ul>
                    </div>
                    <div class="footer-section">
                        <h3>{"📈 Statistiques"}</h3>
                        <p>{"Lois actives: 127"}</p>
                        <p>{"Utilisateurs: 2,456"}</p>
                        <p>{"Votes cette semaine: 834"}</p>
                        <p>{"Dernière mise à jour: aujourd'hui"}</p>
                    </div>
                </div>
                <div class="footer-bottom">
                    <p>{"© 2024 E-Government Blockchain. Système open-source sous licence MIT."}</p>
                </div>
            </footer>
        </div>
    }
}
