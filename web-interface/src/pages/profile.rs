use yew::prelude::*;
use crate::pages::auth::UserAccount;
use web_sys::HtmlInputElement;
use gloo::storage::{LocalStorage, Storage};

#[derive(Clone, PartialEq)]
pub struct ProfileForm {
    pub username: String,
    pub email: String,
}

#[derive(Properties, PartialEq)]
pub struct ProfileProps {
    pub user: Option<UserAccount>,
    pub on_logout: Callback<()>,
    pub on_navigate: Callback<String>,
}

#[function_component(ProfilePage)]
pub fn profile_page(props: &ProfileProps) -> Html {
    let profile_form = use_state(|| ProfileForm {
        username: String::new(),
        email: String::new(),
    });
    let is_editing = use_state(|| false);
    let is_saving = use_state(|| false);
    let success_message = use_state(|| Option::<String>::None);
    let error_message = use_state(|| Option::<String>::None);

    // Si l'utilisateur n'est pas connecté
    if props.user.is_none() {
        return html! {
            <div class="profile-container">
                <div class="auth-required">
                    <h2>{"🔐 Connexion requise"}</h2>
                    <p>{"Vous devez être connecté pour accéder à votre profil."}</p>
                    <button 
                        class="btn-primary"
                        onclick={
                            let on_navigate = props.on_navigate.clone();
                            Callback::from(move |_| on_navigate.emit("login".to_string()))
                        }
                    >
                        {"Se connecter"}
                    </button>
                </div>
            </div>
        };
    }

    let user = props.user.as_ref().unwrap();

    // Initialiser le formulaire avec les données utilisateur
    use_effect_with(user.clone(), {
        let profile_form = profile_form.clone();
        move |user| {
            profile_form.set(ProfileForm {
                username: user.username.clone(),
                email: user.email.clone(),
            });
        }
    });

    let on_edit_toggle = {
        let is_editing = is_editing.clone();
        let profile_form = profile_form.clone();
        let user = user.clone();
        
        Callback::from(move |_| {
            let editing = !*is_editing;
            is_editing.set(editing);
            
            if !editing {
                // Annuler les modifications
                profile_form.set(ProfileForm {
                    username: user.username.clone(),
                    email: user.email.clone(),
                });
            }
        })
    };

    let on_username_change = {
        let profile_form = profile_form.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut form = (*profile_form).clone();
            form.username = input.value();
            profile_form.set(form);
        })
    };

    let on_email_change = {
        let profile_form = profile_form.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut form = (*profile_form).clone();
            form.email = input.value();
            profile_form.set(form);
        })
    };

    let on_save = {
        let profile_form = profile_form.clone();
        let is_editing = is_editing.clone();
        let is_saving = is_saving.clone();
        let success_message = success_message.clone();
        let error_message = error_message.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            
            let form = &*profile_form;
            
            // Validation
            if form.username.trim().is_empty() {
                error_message.set(Some("Le nom d'utilisateur est obligatoire".to_string()));
                return;
            }
            
            if form.email.trim().is_empty() || !form.email.contains('@') {
                error_message.set(Some("Adresse email valide requise".to_string()));
                return;
            }
            
            is_saving.set(true);
            error_message.set(None);
            
            // Simulation de la sauvegarde
            let is_saving_clone = is_saving.clone();
            let is_editing_clone = is_editing.clone();
            let success_message_clone = success_message.clone();
            
            wasm_bindgen_futures::spawn_local(async move {
                gloo::timers::future::TimeoutFuture::new(1000).await;
                
                // En réalité, cela ferait un appel API pour mettre à jour l'utilisateur
                // et mettrait à jour le localStorage
                
                success_message_clone.set(Some("Profil mis à jour avec succès !".to_string()));
                is_editing_clone.set(false);
                is_saving_clone.set(false);
            });
        })
    };

    let on_logout = {
        let on_logout = props.on_logout.clone();
        Callback::from(move |_| {
            // Supprimer les données utilisateur du localStorage
            let _ = LocalStorage::delete("current_user");
            on_logout.emit(());
        })
    };

    html! {
        <div class="profile-container">
            <div class="profile-header">
                <h1>{"👤 Mon Profil"}</h1>
                <p>{"Gérez vos informations personnelles et vos préférences"}</p>
            </div>

            if let Some(success) = (*success_message).as_ref() {
                <div class="success-message">
                    <strong>{"✅ "}</strong> {success}
                </div>
            }

            if let Some(error) = (*error_message).as_ref() {
                <div class="error-message">
                    <strong>{"❌ "}</strong> {error}
                </div>
            }

            <div class="profile-content">
                <div class="profile-main">
                    <div class="profile-card">
                        <div class="profile-avatar">
                            <div class="avatar-circle">
                                {user.username.chars().next().unwrap_or('?').to_uppercase().to_string()}
                            </div>
                        </div>
                        
                        <form class="profile-form" onsubmit={on_save}>
                            <div class="form-group">
                                <label for="username">{"Nom d'utilisateur"}</label>
                                <input 
                                    type="text"
                                    id="username"
                                    value={profile_form.username.clone()}
                                    onchange={on_username_change}
                                    disabled={!*is_editing}
                                    required=true
                                />
                            </div>
                            
                            <div class="form-group">
                                <label for="email">{"Adresse email"}</label>
                                <input 
                                    type="email"
                                    id="email"
                                    value={profile_form.email.clone()}
                                    onchange={on_email_change}
                                    disabled={!*is_editing}
                                    required=true
                                />
                            </div>
                            
                            <div class="form-group">
                                <label>{"Clé publique"}</label>
                                <div class="public-key">
                                    <code>{&user.public_key}</code>
                                    <small>{"Cette clé publique vous identifie de manière unique sur la blockchain"}</small>
                                </div>
                            </div>
                            
                            <div class="form-actions">
                                if *is_editing {
                                    <button type="button" class="btn-secondary" onclick={on_edit_toggle.clone()}>
                                        {"Annuler"}
                                    </button>
                                    <button type="submit" class="btn-primary" disabled={*is_saving}>
                                        if *is_saving {
                                            {"💾 Sauvegarde..."}
                                        } else {
                                            {"💾 Sauvegarder"}
                                        }
                                    </button>
                                } else {
                                    <button type="button" class="btn-primary" onclick={on_edit_toggle}>
                                        {"✏️ Modifier"}
                                    </button>
                                }
                            </div>
                        </form>
                    </div>
                </div>

                <div class="profile-sidebar">
                    <div class="stats-card">
                        <h3>{"📊 Vos statistiques"}</h3>
                        <div class="stat-item">
                            <span class="stat-label">{"Réputation:"}</span>
                            <span class="stat-value">{user.reputation}</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">{"Membre depuis:"}</span>
                            <span class="stat-value">{"15 jours"}</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">{"Votes effectués:"}</span>
                            <span class="stat-value">{"12"}</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">{"Propositions soumises:"}</span>
                            <span class="stat-value">{"3"}</span>
                        </div>
                    </div>

                    <div class="reputation-card">
                        <h3>{"🏆 Système de réputation"}</h3>
                        <div class="reputation-bar">
                            <div class="reputation-fill" style={format!("width: {}%", (user.reputation as f32 / 200.0 * 100.0).min(100.0))}></div>
                        </div>
                        <p class="reputation-text">
                            {format!("{} / 200 points", user.reputation)}
                        </p>
                        <div class="reputation-info">
                            <h4>{"Comment gagner des points:"}</h4>
                            <ul>
                                <li>{"Voter sur les propositions: +2 points"}</li>
                                <li>{"Proposer une loi acceptée: +20 points"}</li>
                                <li>{"Participation active: +5 points/mois"}</li>
                                <li>{"Respect des règles: bonus variable"}</li>
                            </ul>
                        </div>
                    </div>

                    <div class="activity-card">
                        <h3>{"🕒 Activité récente"}</h3>
                        <div class="activity-list">
                            <div class="activity-item">
                                <span class="activity-icon">{"🗳️"}</span>
                                <div class="activity-content">
                                    <p>{"Vote sur 'Semaine de 4 jours'"}</p>
                                    <small>{"Il y a 2 heures"}</small>
                                </div>
                            </div>
                            <div class="activity-item">
                                <span class="activity-icon">{"✏️"}</span>
                                <div class="activity-content">
                                    <p>{"Proposition soumise"}</p>
                                    <small>{"Il y a 1 jour"}</small>
                                </div>
                            </div>
                            <div class="activity-item">
                                <span class="activity-icon">{"📜"}</span>
                                <div class="activity-content">
                                    <p>{"Consultation de 3 lois"}</p>
                                    <small>{"Il y a 3 jours"}</small>
                                </div>
                            </div>
                        </div>
                    </div>

                    <div class="actions-card">
                        <h3>{"⚙️ Actions"}</h3>
                        <div class="action-buttons">
                            <button 
                                class="btn-secondary full-width"
                                onclick={
                                    let on_navigate = props.on_navigate.clone();
                                    Callback::from(move |_| on_navigate.emit("voting".to_string()))
                                }
                            >
                                {"🗳️ Aller aux votes"}
                            </button>
                            <button 
                                class="btn-secondary full-width"
                                onclick={
                                    let on_navigate = props.on_navigate.clone();
                                    Callback::from(move |_| on_navigate.emit("proposals".to_string()))
                                }
                            >
                                {"✏️ Faire une proposition"}
                            </button>
                            <button 
                                class="btn-secondary full-width"
                                onclick={
                                    let on_navigate = props.on_navigate.clone();
                                    Callback::from(move |_| on_navigate.emit("laws".to_string()))
                                }
                            >
                                {"📜 Consulter les lois"}
                            </button>
                        </div>
                    </div>

                    <div class="logout-section">
                        <button class="btn-danger full-width" onclick={on_logout}>
                            {"🚪 Se déconnecter"}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}