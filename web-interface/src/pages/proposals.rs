use yew::prelude::*;
use web_sys::HtmlInputElement;
use common::{LawCategory, LawStatus};
use crate::pages::auth::UserAccount;
use uuid::Uuid;

#[derive(Clone, PartialEq)]
pub struct ProposalForm {
    pub title: String,
    pub content: String,
    pub category: LawCategory,
    pub modification_target: Option<Uuid>, // Si c'est une modification d'une loi existante
    pub justification: String,
}

impl Default for ProposalForm {
    fn default() -> Self {
        Self {
            title: String::new(),
            content: String::new(),
            category: LawCategory::Civil,
            modification_target: None,
            justification: String::new(),
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct ProposalsProps {
    pub user: Option<UserAccount>,
    pub on_navigate: Callback<String>,
}

#[function_component(ProposalsPage)]
pub fn proposals_page(props: &ProposalsProps) -> Html {
    let proposal_form = use_state(ProposalForm::default);
    let is_submitting = use_state(|| false);
    let success_message = use_state(|| Option::<String>::None);
    let error_message = use_state(|| Option::<String>::None);
    let proposal_type = use_state(|| "new"); // "new" ou "amendment"

    // Si l'utilisateur n'est pas connecté
    if props.user.is_none() {
        return html! {
            <div class="proposals-container">
                <div class="auth-required">
                    <h2>{"🔐 Connexion requise"}</h2>
                    <p>{"Vous devez être connecté pour proposer des modifications de loi."}</p>
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

    let on_title_change = {
        let proposal_form = proposal_form.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut form = (*proposal_form).clone();
            form.title = input.value();
            proposal_form.set(form);
        })
    };

    let on_content_change = {
        let proposal_form = proposal_form.clone();
        Callback::from(move |e: Event| {
            let textarea: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            let mut form = (*proposal_form).clone();
            form.content = textarea.value();
            proposal_form.set(form);
        })
    };

    let on_justification_change = {
        let proposal_form = proposal_form.clone();
        Callback::from(move |e: Event| {
            let textarea: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
            let mut form = (*proposal_form).clone();
            form.justification = textarea.value();
            proposal_form.set(form);
        })
    };

    let on_category_change = {
        let proposal_form = proposal_form.clone();
        Callback::from(move |category: LawCategory| {
            let mut form = (*proposal_form).clone();
            form.category = category;
            proposal_form.set(form);
        })
    };

    let on_proposal_type_change = {
        let proposal_type = proposal_type.clone();
        let proposal_form = proposal_form.clone();
        Callback::from(move |new_type: &str| {
            proposal_type.set(new_type);
            if new_type == "new" {
                let mut form = (*proposal_form).clone();
                form.modification_target = None;
                proposal_form.set(form);
            }
        })
    };

    let on_submit = {
        let proposal_form = proposal_form.clone();
        let is_submitting = is_submitting.clone();
        let success_message = success_message.clone();
        let error_message = error_message.clone();
        let user = user.clone();
        
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            
            let form = &*proposal_form;
            
            // Validation
            if form.title.trim().is_empty() {
                error_message.set(Some("Le titre est obligatoire".to_string()));
                return;
            }
            
            if form.content.trim().is_empty() {
                error_message.set(Some("Le contenu est obligatoire".to_string()));
                return;
            }
            
            if form.title.len() < 10 {
                error_message.set(Some("Le titre doit contenir au moins 10 caractères".to_string()));
                return;
            }
            
            if form.content.len() < 50 {
                error_message.set(Some("Le contenu doit contenir au moins 50 caractères".to_string()));
                return;
            }
            
            if form.modification_target.is_some() && form.justification.trim().is_empty() {
                error_message.set(Some("Une justification est requise pour les amendements".to_string()));
                return;
            }
            
            is_submitting.set(true);
            error_message.set(None);
            
            // Simulation de la soumission (remplacer par API call)
            // Dans un vrai système, cela créerait une transaction blockchain
            
            // Simuler un délai réseau
            let is_submitting_clone = is_submitting.clone();
            let success_message_clone = success_message.clone();
            let proposal_form_clone = proposal_form.clone();
            
            wasm_bindgen_futures::spawn_local(async move {
                gloo::timers::future::TimeoutFuture::new(1500).await;
                
                success_message_clone.set(Some(format!(
                    "Proposition '{}' soumise avec succès ! Elle sera examinée par la communauté.",
                    form.title
                )));
                
                // Reset du formulaire
                proposal_form_clone.set(ProposalForm::default());
                is_submitting_clone.set(false);
            });
        })
    };

    html! {
        <div class="proposals-container">
            <div class="proposals-header">
                <h1>{"✏️ Proposer une modification"}</h1>
                <p>{"Contribuez à l'amélioration de notre système légal"}</p>
            </div>

            <div class="user-info">
                <h3>{format!("Connecté en tant que: {}", user.username)}</h3>
                <p>{format!("Réputation: {} points", user.reputation)}</p>
            </div>

            // Messages de succès/erreur
            if let Some(success) = (*success_message).as_ref() {
                <div class="success-message">
                    <strong>{"✅ Succès: "}</strong> {success}
                </div>
            }

            if let Some(error) = (*error_message).as_ref() {
                <div class="error-message">
                    <strong>{"❌ Erreur: "}</strong> {error}
                </div>
            }

            // Sélection du type de proposition
            <div class="proposal-type-selector">
                <h3>{"Type de proposition"}</h3>
                <div class="type-buttons">
                    <button 
                        class={if *proposal_type == "new" { "type-btn active" } else { "type-btn" }}
                        onclick={
                            let on_change = on_proposal_type_change.clone();
                            Callback::from(move |_| on_change.emit("new"))
                        }
                    >
                        <div class="type-icon">{"📜"}</div>
                        <div class="type-info">
                            <h4>{"Nouvelle loi"}</h4>
                            <p>{"Proposer un nouveau texte de loi"}</p>
                        </div>
                    </button>
                    <button 
                        class={if *proposal_type == "amendment" { "type-btn active" } else { "type-btn" }}
                        onclick={
                            let on_change = on_proposal_type_change.clone();
                            Callback::from(move |_| on_change.emit("amendment"))
                        }
                    >
                        <div class="type-icon">{"🔧"}</div>
                        <div class="type-info">
                            <h4>{"Amendement"}</h4>
                            <p>{"Modifier une loi existante"}</p>
                        </div>
                    </button>
                </div>
            </div>

            // Formulaire de proposition
            <form class="proposal-form" onsubmit={on_submit}>
                <div class="form-section">
                    <h3>{"Informations générales"}</h3>
                    
                    <div class="form-group">
                        <label for="title">{"Titre de la proposition *"}</label>
                        <input 
                            type="text"
                            id="title"
                            value={proposal_form.title.clone()}
                            onchange={on_title_change}
                            placeholder="Ex: Loi sur la protection de l'environnement urbain"
                            required=true
                            maxlength="200"
                        />
                        <small class="form-help">{"Soyez précis et descriptif (minimum 10 caractères)"}</small>
                    </div>

                    <div class="form-group">
                        <label for="category">{"Catégorie *"}</label>
                        <div class="category-buttons">
                            <button 
                                type="button"
                                class={if proposal_form.category == LawCategory::Constitutional { "category-btn active" } else { "category-btn" }}
                                onclick={
                                    let on_change = on_category_change.clone();
                                    Callback::from(move |_| on_change.emit(LawCategory::Constitutional))
                                }
                            >
                                {"Constitutionnel"}
                            </button>
                            <button 
                                type="button"
                                class={if proposal_form.category == LawCategory::Civil { "category-btn active" } else { "category-btn" }}
                                onclick={
                                    let on_change = on_category_change.clone();
                                    Callback::from(move |_| on_change.emit(LawCategory::Civil))
                                }
                            >
                                {"Civil"}
                            </button>
                            <button 
                                type="button"
                                class={if proposal_form.category == LawCategory::Criminal { "category-btn active" } else { "category-btn" }}
                                onclick={
                                    let on_change = on_category_change.clone();
                                    Callback::from(move |_| on_change.emit(LawCategory::Criminal))
                                }
                            >
                                {"Pénal"}
                            </button>
                            <button 
                                type="button"
                                class={if proposal_form.category == LawCategory::Economic { "category-btn active" } else { "category-btn" }}
                                onclick={
                                    let on_change = on_category_change.clone();
                                    Callback::from(move |_| on_change.emit(LawCategory::Economic))
                                }
                            >
                                {"Économique"}
                            </button>
                        </div>
                    </div>
                </div>

                if *proposal_type == "amendment" {
                    <div class="form-section">
                        <h3>{"Loi à modifier"}</h3>
                        <div class="form-group">
                            <label>{"Sélectionner la loi à modifier"}</label>
                            <button 
                                type="button"
                                class="btn-secondary"
                                onclick={
                                    let on_navigate = props.on_navigate.clone();
                                    Callback::from(move |_| on_navigate.emit("laws".to_string()))
                                }
                            >
                                {"📋 Parcourir les lois existantes"}
                            </button>
                            <small class="form-help">{"Cliquez pour sélectionner une loi existante à modifier"}</small>
                        </div>
                    </div>
                }

                <div class="form-section">
                    <h3>{"Contenu de la proposition"}</h3>
                    
                    <div class="form-group">
                        <label for="content">{"Texte de la proposition *"}</label>
                        <textarea 
                            id="content"
                            value={proposal_form.content.clone()}
                            onchange={on_content_change}
                            placeholder={if *proposal_type == "new" {
                                "Rédigez ici le texte complet de votre proposition de loi.\n\nStructurez votre texte avec des articles:\nArticle 1: ...\nArticle 2: ..."
                            } else {
                                "Décrivez précisément les modifications que vous souhaitez apporter à la loi existante."
                            }}
                            required=true
                            rows="12"
                        />
                        <small class="form-help">{"Minimum 50 caractères. Utilisez un langage clair et précis."}</small>
                    </div>

                    if *proposal_type == "amendment" {
                        <div class="form-group">
                            <label for="justification">{"Justification de l'amendement *"}</label>
                            <textarea 
                                id="justification"
                                value={proposal_form.justification.clone()}
                                onchange={on_justification_change}
                                placeholder="Expliquez pourquoi cette modification est nécessaire. Quels problèmes résout-elle ? Quels sont les bénéfices attendus ?"
                                required=true
                                rows="6"
                            />
                            <small class="form-help">{"Justifiez clairement pourquoi cette modification est nécessaire."}</small>
                        </div>
                    }
                </div>

                <div class="form-section">
                    <h3>{"Informations importantes"}</h3>
                    <div class="info-box">
                        <h4>{"📋 Processus de validation"}</h4>
                        <ul>
                            <li>{"Votre proposition sera examinée par la communauté"}</li>
                            <li>{"Les citoyens pourront voter pour ou contre"}</li>
                            <li>{"Un quorum minimum est requis pour la validité"}</li>
                            <li>{"Les propositions approuvées deviennent des lois actives"}</li>
                        </ul>
                    </div>

                    <div class="info-box">
                        <h4>{"🎯 Conseils pour une bonne proposition"}</h4>
                        <ul>
                            <li>{"Soyez clair et précis dans la rédaction"}</li>
                            <li>{"Justifiez la nécessité de votre proposition"}</li>
                            <li>{"Considérez les impacts sur la société"}</li>
                            <li>{"Respectez les droits fondamentaux"}</li>
                        </ul>
                    </div>
                </div>

                <div class="form-actions">
                    <button 
                        type="button" 
                        class="btn-secondary"
                        onclick={
                            let proposal_form = proposal_form.clone();
                            Callback::from(move |_| proposal_form.set(ProposalForm::default()))
                        }
                    >
                        {"🔄 Réinitialiser"}
                    </button>
                    
                    <button 
                        type="submit" 
                        class="btn-primary"
                        disabled={*is_submitting}
                    >
                        if *is_submitting {
                            {"📤 Envoi en cours..."}
                        } else {
                            {"📤 Soumettre la proposition"}
                        }
                    </button>
                </div>
            </form>
        </div>
    }
}