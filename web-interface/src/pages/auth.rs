use yew::prelude::*;
use web_sys::HtmlInputElement;
use gloo::storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct UserAccount {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub public_key: String,
    pub reputation: i32,
}

#[derive(Clone, PartialEq)]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

#[derive(Clone, PartialEq)]
pub struct RegisterData {
    pub username: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Properties, PartialEq)]
pub struct AuthProps {
    pub on_login: Callback<UserAccount>,
}

#[function_component(LoginPage)]
pub fn login_page(props: &AuthProps) -> Html {
    let login_data = use_state(|| LoginData {
        username: String::new(),
        password: String::new(),
    });
    
    let error_msg = use_state(|| Option::<String>::None);
    let is_loading = use_state(|| false);

    let on_username_change = {
        let login_data = login_data.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut data = (*login_data).clone();
            data.username = input.value();
            login_data.set(data);
        })
    };

    let on_password_change = {
        let login_data = login_data.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut data = (*login_data).clone();
            data.password = input.value();
            login_data.set(data);
        })
    };

    let on_submit = {
        let login_data = login_data.clone();
        let error_msg = error_msg.clone();
        let is_loading = is_loading.clone();
        let on_login = props.on_login.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            
            if login_data.username.is_empty() || login_data.password.is_empty() {
                error_msg.set(Some("Veuillez remplir tous les champs".to_string()));
                return;
            }
            
            is_loading.set(true);
            error_msg.set(None);
            
            // Simulation de l'authentification (remplacer par API call)
            // Pour la démo, créons un utilisateur mock
            let user = UserAccount {
                id: Uuid::new_v4(),
                username: login_data.username.clone(),
                email: format!("{}@example.com", login_data.username),
                public_key: "mock_public_key".to_string(),
                reputation: 100,
            };
            
            // Sauvegarder en localStorage pour persistance
            if let Ok(_) = LocalStorage::set("current_user", &user) {
                on_login.emit(user);
            } else {
                error_msg.set(Some("Erreur de stockage local".to_string()));
            }
            
            is_loading.set(false);
        })
    };

    html! {
        <div class="auth-container">
            <div class="auth-card">
                <div class="auth-header">
                    <h2>{"🔐 Connexion"}</h2>
                    <p>{"Connectez-vous à votre compte e-gouvernement"}</p>
                </div>
                
                <form {onsubmit} class="auth-form">
                    <div class="form-group">
                        <label for="username">{"Nom d'utilisateur"}</label>
                        <input
                            type="text"
                            id="username"
                            value={login_data.username.clone()}
                            onchange={on_username_change}
                            placeholder="Votre nom d'utilisateur"
                            required=true
                        />
                    </div>
                    
                    <div class="form-group">
                        <label for="password">{"Mot de passe"}</label>
                        <input
                            type="password"
                            id="password"
                            value={login_data.password.clone()}
                            onchange={on_password_change}
                            placeholder="Votre mot de passe"
                            required=true
                        />
                    </div>
                    
                    if let Some(error) = (*error_msg).as_ref() {
                        <div class="error-message">
                            {error}
                        </div>
                    }
                    
                    <button 
                        type="submit" 
                        class="btn-primary"
                        disabled={*is_loading}
                    >
                        if *is_loading {
                            {"Connexion..."}
                        } else {
                            {"Se connecter"}
                        }
                    </button>
                </form>
                
                <div class="auth-footer">
                    <p>{"Pas encore de compte ?"} <a href="#register">{"Créer un compte"}</a></p>
                </div>
            </div>
        </div>
    }
}

#[function_component(RegisterPage)]
pub fn register_page(props: &AuthProps) -> Html {
    let register_data = use_state(|| RegisterData {
        username: String::new(),
        email: String::new(),
        password: String::new(),
        confirm_password: String::new(),
    });
    
    let error_msg = use_state(|| Option::<String>::None);
    let is_loading = use_state(|| false);

    let on_username_change = {
        let register_data = register_data.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut data = (*register_data).clone();
            data.username = input.value();
            register_data.set(data);
        })
    };

    let on_email_change = {
        let register_data = register_data.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut data = (*register_data).clone();
            data.email = input.value();
            register_data.set(data);
        })
    };

    let on_password_change = {
        let register_data = register_data.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut data = (*register_data).clone();
            data.password = input.value();
            register_data.set(data);
        })
    };

    let on_confirm_password_change = {
        let register_data = register_data.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut data = (*register_data).clone();
            data.confirm_password = input.value();
            register_data.set(data);
        })
    };

    let on_submit = {
        let register_data = register_data.clone();
        let error_msg = error_msg.clone();
        let is_loading = is_loading.clone();
        let on_login = props.on_login.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            
            let data = &*register_data;
            
            // Validation
            if data.username.is_empty() || data.email.is_empty() || 
               data.password.is_empty() || data.confirm_password.is_empty() {
                error_msg.set(Some("Veuillez remplir tous les champs".to_string()));
                return;
            }
            
            if data.password != data.confirm_password {
                error_msg.set(Some("Les mots de passe ne correspondent pas".to_string()));
                return;
            }
            
            if data.password.len() < 6 {
                error_msg.set(Some("Le mot de passe doit contenir au moins 6 caractères".to_string()));
                return;
            }
            
            if !data.email.contains('@') {
                error_msg.set(Some("Adresse email invalide".to_string()));
                return;
            }
            
            is_loading.set(true);
            error_msg.set(None);
            
            // Simulation de l'enregistrement (remplacer par API call)
            let user = UserAccount {
                id: Uuid::new_v4(),
                username: data.username.clone(),
                email: data.email.clone(),
                public_key: format!("pubkey_{}", Uuid::new_v4()),
                reputation: 50, // Réputation initiale
            };
            
            // Sauvegarder en localStorage pour persistance
            if let Ok(_) = LocalStorage::set("current_user", &user) {
                on_login.emit(user);
            } else {
                error_msg.set(Some("Erreur de stockage local".to_string()));
            }
            
            is_loading.set(false);
        })
    };

    html! {
        <div class="auth-container">
            <div class="auth-card">
                <div class="auth-header">
                    <h2>{"📝 Créer un compte"}</h2>
                    <p>{"Rejoignez la démocratie numérique"}</p>
                </div>
                
                <form onsubmit={on_submit} class="auth-form">
                    <div class="form-group">
                        <label for="username">{"Nom d'utilisateur"}</label>
                        <input
                            type="text"
                            id="username"
                            value={register_data.username.clone()}
                            onchange={on_username_change}
                            placeholder="Choisissez un nom d'utilisateur"
                            required=true
                        />
                    </div>
                    
                    <div class="form-group">
                        <label for="email">{"Adresse email"}</label>
                        <input
                            type="email"
                            id="email"
                            value={register_data.email.clone()}
                            onchange={on_email_change}
                            placeholder="votre@email.com"
                            required=true
                        />
                    </div>
                    
                    <div class="form-group">
                        <label for="password">{"Mot de passe"}</label>
                        <input
                            type="password"
                            id="password"
                            value={register_data.password.clone()}
                            onchange={on_password_change}
                            placeholder="Au moins 6 caractères"
                            required=true
                        />
                    </div>
                    
                    <div class="form-group">
                        <label for="confirm_password">{"Confirmer le mot de passe"}</label>
                        <input
                            type="password"
                            id="confirm_password"
                            value={register_data.confirm_password.clone()}
                            onchange={on_confirm_password_change}
                            placeholder="Répétez votre mot de passe"
                            required=true
                        />
                    </div>
                    
                    if let Some(error) = (*error_msg).as_ref() {
                        <div class="error-message">
                            {error}
                        </div>
                    }
                    
                    <button 
                        type="submit" 
                        class="btn-primary"
                        disabled={*is_loading}
                    >
                        if *is_loading {
                            {"Création..."}
                        } else {
                            {"Créer le compte"}
                        }
                    </button>
                </form>
                
                <div class="auth-footer">
                    <p>{"Déjà un compte ?"} <a href="#login">{"Se connecter"}</a></p>
                </div>
            </div>
        </div>
    }
}