use yew::prelude::*;
use common::{Law, LawStatus, LawCategory};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use web_sys::HtmlInputElement;

#[derive(Clone, PartialEq)]
pub struct LawFilter {
    pub search_term: String,
    pub category: Option<LawCategory>,
    pub status: Option<LawStatus>,
}

#[derive(Properties, PartialEq)]
pub struct LawsProps {
    pub on_navigate: Callback<String>,
}

#[function_component(LawsPage)]
pub fn laws_page(props: &LawsProps) -> Html {
    let laws = use_state(|| get_mock_laws());
    let filter = use_state(|| LawFilter {
        search_term: String::new(),
        category: None,
        status: None,
    });
    let selected_law = use_state(|| Option::<Law>::None);

    let filtered_laws = {
        let laws = laws.clone();
        let filter = filter.clone();
        
        (*laws).iter()
            .filter(|law| {
                let matches_search = filter.search_term.is_empty() || 
                    law.title.to_lowercase().contains(&filter.search_term.to_lowercase()) ||
                    law.content.to_lowercase().contains(&filter.search_term.to_lowercase());
                
                let matches_category = filter.category.is_none() || 
                    filter.category.as_ref() == Some(&law.category);
                
                let matches_status = filter.status.is_none() || 
                    filter.status.as_ref() == Some(&law.status);
                
                matches_search && matches_category && matches_status
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    let on_search_change = {
        let filter = filter.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let mut new_filter = (*filter).clone();
            new_filter.search_term = input.value();
            filter.set(new_filter);
        })
    };

    let on_category_change = {
        let filter = filter.clone();
        Callback::from(move |category: Option<LawCategory>| {
            let mut new_filter = (*filter).clone();
            new_filter.category = category;
            filter.set(new_filter);
        })
    };

    let on_status_change = {
        let filter = filter.clone();
        Callback::from(move |status: Option<LawStatus>| {
            let mut new_filter = (*filter).clone();
            new_filter.status = status;
            filter.set(new_filter);
        })
    };

    let on_law_select = {
        let selected_law = selected_law.clone();
        Callback::from(move |law: Law| {
            selected_law.set(Some(law));
        })
    };

    let on_close_detail = {
        let selected_law = selected_law.clone();
        Callback::from(move |_| {
            selected_law.set(None);
        })
    };

    html! {
        <div class="laws-container">
            <div class="laws-header">
                <h1>{"📜 Consultation des lois"}</h1>
                <p>{"Explorez et consultez les textes de loi en vigueur et en discussion"}</p>
            </div>

            // Filtres
            <div class="laws-filters">
                <div class="search-bar">
                    <input 
                        type="text"
                        placeholder="Rechercher une loi..."
                        value={filter.search_term.clone()}
                        onchange={on_search_change}
                        class="search-input"
                    />
                </div>
                
                <div class="filter-buttons">
                    <div class="filter-group">
                        <span class="filter-label">{"Catégorie:"}</span>
                        <button 
                            class={if filter.category.is_none() { "filter-btn active" } else { "filter-btn" }}
                            onclick={
                                let on_category_change = on_category_change.clone();
                                Callback::from(move |_| on_category_change.emit(None))
                            }
                        >
                            {"Toutes"}
                        </button>
                        <button 
                            class={if filter.category == Some(LawCategory::Constitutional) { "filter-btn active" } else { "filter-btn" }}
                            onclick={
                                let on_category_change = on_category_change.clone();
                                Callback::from(move |_| on_category_change.emit(Some(LawCategory::Constitutional)))
                            }
                        >
                            {"Constitutionnel"}
                        </button>
                        <button 
                            class={if filter.category == Some(LawCategory::Civil) { "filter-btn active" } else { "filter-btn" }}
                            onclick={
                                let on_category_change = on_category_change.clone();
                                Callback::from(move |_| on_category_change.emit(Some(LawCategory::Civil)))
                            }
                        >
                            {"Civil"}
                        </button>
                        <button 
                            class={if filter.category == Some(LawCategory::Criminal) { "filter-btn active" } else { "filter-btn" }}
                            onclick={
                                let on_category_change = on_category_change.clone();
                                Callback::from(move |_| on_category_change.emit(Some(LawCategory::Criminal)))
                            }
                        >
                            {"Pénal"}
                        </button>
                        <button 
                            class={if filter.category == Some(LawCategory::Economic) { "filter-btn active" } else { "filter-btn" }}
                            onclick={
                                let on_category_change = on_category_change.clone();
                                Callback::from(move |_| on_category_change.emit(Some(LawCategory::Economic)))
                            }
                        >
                            {"Économique"}
                        </button>
                    </div>
                    
                    <div class="filter-group">
                        <span class="filter-label">{"Statut:"}</span>
                        <button 
                            class={if filter.status.is_none() { "filter-btn active" } else { "filter-btn" }}
                            onclick={
                                let on_status_change = on_status_change.clone();
                                Callback::from(move |_| on_status_change.emit(None))
                            }
                        >
                            {"Tous"}
                        </button>
                        <button 
                            class={if filter.status == Some(LawStatus::Active) { "filter-btn active" } else { "filter-btn" }}
                            onclick={
                                let on_status_change = on_status_change.clone();
                                Callback::from(move |_| on_status_change.emit(Some(LawStatus::Active)))
                            }
                        >
                            {"Actif"}
                        </button>
                        <button 
                            class={if filter.status == Some(LawStatus::UnderReview) { "filter-btn active" } else { "filter-btn" }}
                            onclick={
                                let on_status_change = on_status_change.clone();
                                Callback::from(move |_| on_status_change.emit(Some(LawStatus::UnderReview)))
                            }
                        >
                            {"En révision"}
                        </button>
                        <button 
                            class={if filter.status == Some(LawStatus::Proposed) { "filter-btn active" } else { "filter-btn" }}
                            onclick={
                                let on_status_change = on_status_change.clone();
                                Callback::from(move |_| on_status_change.emit(Some(LawStatus::Proposed)))
                            }
                        >
                            {"Proposé"}
                        </button>
                    </div>
                </div>
            </div>

            // Liste des lois
            <div class="laws-content">
                if filtered_laws.is_empty() {
                    <div class="no-results">
                        <p>{"Aucune loi ne correspond à vos critères de recherche"}</p>
                    </div>
                } else {
                    <div class="laws-list">
                        {for filtered_laws.iter().map(|law| {
                            let law_click = {
                                let law = law.clone();
                                let on_law_select = on_law_select.clone();
                                Callback::from(move |_| on_law_select.emit(law.clone()))
                            };
                            
                            html! {
                                <div class="law-card" onclick={law_click}>
                                    <div class="law-header">
                                        <h3 class="law-title">{&law.title}</h3>
                                        <div class="law-badges">
                                            <span class={format!("badge badge-{}", law.status.to_string().to_lowercase())}>
                                                {law.status.to_string()}
                                            </span>
                                            <span class="badge badge-category">
                                                {law.category.to_string()}
                                            </span>
                                        </div>
                                    </div>
                                    <div class="law-content">
                                        <p class="law-description">
                                            {if law.content.len() > 200 {
                                                format!("{}...", &law.content[..200])
                                            } else {
                                                law.content.clone()
                                            }}
                                        </p>
                                    </div>
                                    <div class="law-footer">
                                        <div class="law-dates">
                                            <span>{"Créé le: "}{law.created_at.format("%d/%m/%Y").to_string()}</span>
                                            if let Some(updated) = law.updated_at {
                                                <span>{" • Modifié le: "}{updated.format("%d/%m/%Y").to_string()}</span>
                                            }
                                        </div>
                                        <div class="law-author">
                                            {"Par: "}{&law.author}
                                        </div>
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                }
            </div>

            // Détail de la loi sélectionnée
            if let Some(law) = (*selected_law).as_ref() {
                <div class="law-detail-overlay" onclick={on_close_detail.clone()}>
                    <div class="law-detail-modal" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                        <div class="law-detail-header">
                            <h2>{&law.title}</h2>
                            <button class="close-btn" onclick={on_close_detail}>{"×"}</button>
                        </div>
                        
                        <div class="law-detail-badges">
                            <span class={format!("badge badge-{}", law.status.to_string().to_lowercase())}>
                                {law.status.to_string()}
                            </span>
                            <span class="badge badge-category">
                                {law.category.to_string()}
                            </span>
                        </div>
                        
                        <div class="law-detail-content">
                            <div class="law-full-text">
                                <h4>{"Texte complet:"}</h4>
                                <div class="law-text">
                                    {law.content.split('\n').map(|paragraph| {
                                        html! { <p>{paragraph}</p> }
                                    }).collect::<Html>()}
                                </div>
                            </div>
                            
                            <div class="law-metadata">
                                <h4>{"Informations:"}</h4>
                                <table class="metadata-table">
                                    <tr>
                                        <td>{"ID:"}</td>
                                        <td>{law.id.to_string()}</td>
                                    </tr>
                                    <tr>
                                        <td>{"Auteur:"}</td>
                                        <td>{&law.author}</td>
                                    </tr>
                                    <tr>
                                        <td>{"Créé le:"}</td>
                                        <td>{law.created_at.format("%d/%m/%Y à %H:%M").to_string()}</td>
                                    </tr>
                                    if let Some(updated) = law.updated_at {
                                        <tr>
                                            <td>{"Modifié le:"}</td>
                                            <td>{updated.format("%d/%m/%Y à %H:%M").to_string()}</td>
                                        </tr>
                                    }
                                    <tr>
                                        <td>{"Version:"}</td>
                                        <td>{law.version}</td>
                                    </tr>
                                </table>
                            </div>
                        </div>
                        
                        <div class="law-detail-actions">
                            <button 
                                class="btn-secondary"
                                onclick={
                                    let props = props.clone();
                                    Callback::from(move |_| props.on_navigate.emit("proposals".to_string()))
                                }
                            >
                                {"✏️ Proposer une modification"}
                            </button>
                            <button 
                                class="btn-primary"
                                onclick={
                                    let props = props.clone();
                                    Callback::from(move |_| props.on_navigate.emit("voting".to_string()))
                                }
                            >
                                {"🗳️ Aller aux votes"}
                            </button>
                        </div>
                    </div>
                </div>
            }
        </div>
    }
}

// Données mock pour le développement
fn get_mock_laws() -> Vec<Law> {
    vec![
        Law {
            id: Uuid::new_v4(),
            title: "Loi sur la protection des données personnelles".to_string(),
            content: "Cette loi établit les principes fondamentaux pour la protection des données à caractère personnel des citoyens.\n\nArticle 1: Tout citoyen a le droit de contrôler l'utilisation de ses données personnelles.\n\nArticle 2: Les organisations doivent obtenir un consentement explicite avant de collecter des données.\n\nArticle 3: Les citoyens ont le droit d'accéder, de modifier et de supprimer leurs données.".to_string(),
            category: LawCategory::Civil,
            status: LawStatus::Active,
            author: "Commission de la vie privée".to_string(),
            created_at: Utc::now() - chrono::Duration::days(120),
            updated_at: Some(Utc::now() - chrono::Duration::days(30)),
            version: 2,
        },
        Law {
            id: Uuid::new_v4(),
            title: "Amendement constitutionnel sur la démocratie numérique".to_string(),
            content: "Cet amendement constitutionnel établit les bases légales de la participation citoyenne via les technologies blockchain.\n\nArticle 1: La blockchain est reconnue comme un moyen valide d'enregistrement des votes citoyens.\n\nArticle 2: Tout citoyen majeur peut participer aux votes via la plateforme numérique officielle.\n\nArticle 3: Les résultats des votes blockchain ont force de loi.".to_string(),
            category: LawCategory::Constitutional,
            status: LawStatus::UnderReview,
            author: "Assemblée constituante numérique".to_string(),
            created_at: Utc::now() - chrono::Duration::days(45),
            updated_at: None,
            version: 1,
        },
        Law {
            id: Uuid::new_v4(),
            title: "Code pénal numérique - Cybercriminalité".to_string(),
            content: "Ce code définit les infractions relatives aux crimes informatiques et leurs sanctions.\n\nArticle 1: Le piratage de systèmes gouvernementaux est passible de 5 à 10 ans d'emprisonnement.\n\nArticle 2: La manipulation de votes électroniques constitue un crime contre la démocratie.\n\nArticle 3: L'usurpation d'identité numérique est sanctionnée par une amende et des travaux d'intérêt général.".to_string(),
            category: LawCategory::Criminal,
            status: LawStatus::Active,
            author: "Ministère de la Justice numérique".to_string(),
            created_at: Utc::now() - chrono::Duration::days(200),
            updated_at: Some(Utc::now() - chrono::Duration::days(15)),
            version: 3,
        },
        Law {
            id: Uuid::new_v4(),
            title: "Proposition: Revenu universel de base".to_string(),
            content: "Cette proposition de loi vise à instituer un revenu universel de base pour tous les citoyens.\n\nArticle 1: Tout citoyen âgé de plus de 18 ans recevra un revenu mensuel de base.\n\nArticle 2: Ce revenu est financé par une taxe sur l'automatisation et les revenus du capital.\n\nArticle 3: Le montant est ajusté annuellement selon l'indice des prix.".to_string(),
            category: LawCategory::Economic,
            status: LawStatus::Proposed,
            author: "Collectif pour le revenu universel".to_string(),
            created_at: Utc::now() - chrono::Duration::days(10),
            updated_at: None,
            version: 1,
        },
        Law {
            id: Uuid::new_v4(),
            title: "Loi sur la transparence gouvernementale".to_string(),
            content: "Cette loi garantit l'accès du public aux informations gouvernementales.\n\nArticle 1: Toutes les décisions gouvernementales doivent être publiées dans les 48 heures.\n\nArticle 2: Les citoyens ont le droit d'accéder aux documents administratifs.\n\nArticle 3: Les budgets publics doivent être détaillés et accessibles en ligne.".to_string(),
            category: LawCategory::Constitutional,
            status: LawStatus::Active,
            author: "Mouvement pour la transparence".to_string(),
            created_at: Utc::now() - chrono::Duration::days(90),
            updated_at: None,
            version: 1,
        },
    ]
}