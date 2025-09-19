use yew::prelude::*;
use common::{Law, LawStatus, LawCategory, Vote, VoteType};
use crate::pages::auth::UserAccount;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Clone, PartialEq)]
pub struct VotingProposal {
    pub law: Law,
    pub votes_for: u32,
    pub votes_against: u32,
    pub votes_abstain: u32,
    pub total_eligible_voters: u32,
    pub voting_deadline: DateTime<Utc>,
    pub user_vote: Option<VoteType>,
}

#[derive(Properties, PartialEq)]
pub struct VotingProps {
    pub user: Option<UserAccount>,
    pub on_navigate: Callback<String>,
}

#[function_component(VotingPage)]
pub fn voting_page(props: &VotingProps) -> Html {
    let voting_proposals = use_state(|| get_mock_voting_proposals());
    let selected_proposal = use_state(|| Option::<VotingProposal>::None);
    let is_voting = use_state(|| false);
    let vote_message = use_state(|| Option::<String>::None);

    // Si l'utilisateur n'est pas connecté
    if props.user.is_none() {
        return html! {
            <div class="voting-container">
                <div class="auth-required">
                    <h2>{"🔐 Connexion requise"}</h2>
                    <p>{"Vous devez être connecté pour participer aux votes."}</p>
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

    let on_vote = {
        let voting_proposals = voting_proposals.clone();
        let is_voting = is_voting.clone();
        let vote_message = vote_message.clone();
        let user = user.clone();
        
        Callback::from(move |data: (Uuid, VoteType)| {
            let (proposal_id, vote_type) = data;
            
            is_voting.set(true);
            vote_message.set(None);
            
            // Simulation du vote (remplacer par API call)
            let voting_proposals_clone = voting_proposals.clone();
            let is_voting_clone = is_voting.clone();
            let vote_message_clone = vote_message.clone();
            
            wasm_bindgen_futures::spawn_local(async move {
                gloo::timers::future::TimeoutFuture::new(800).await;
                
                // Mettre à jour les propositions avec le nouveau vote
                let mut proposals = (*voting_proposals_clone).clone();
                
                if let Some(proposal) = proposals.iter_mut().find(|p| p.law.id == proposal_id) {
                    // Enlever l'ancien vote s'il existe
                    if let Some(old_vote) = &proposal.user_vote {
                        match old_vote {
                            VoteType::For => proposal.votes_for -= 1,
                            VoteType::Against => proposal.votes_against -= 1,
                            VoteType::Abstain => proposal.votes_abstain -= 1,
                        }
                    }
                    
                    // Ajouter le nouveau vote
                    match vote_type {
                        VoteType::For => proposal.votes_for += 1,
                        VoteType::Against => proposal.votes_against += 1,
                        VoteType::Abstain => proposal.votes_abstain += 1,
                    }
                    
                    proposal.user_vote = Some(vote_type.clone());
                    
                    vote_message_clone.set(Some(format!(
                        "Vote {} enregistré avec succès !",
                        match vote_type {
                            VoteType::For => "POUR",
                            VoteType::Against => "CONTRE", 
                            VoteType::Abstain => "ABSTENTION",
                        }
                    )));
                }
                
                voting_proposals_clone.set(proposals);
                is_voting_clone.set(false);
            });
        })
    };

    let on_proposal_select = {
        let selected_proposal = selected_proposal.clone();
        Callback::from(move |proposal: VotingProposal| {
            selected_proposal.set(Some(proposal));
        })
    };

    let on_close_detail = {
        let selected_proposal = selected_proposal.clone();
        Callback::from(move |_| {
            selected_proposal.set(None);
        })
    };

    html! {
        <div class="voting-container">
            <div class="voting-header">
                <h1>{"🗳️ Votes en cours"}</h1>
                <p>{"Participez aux décisions démocratiques de notre société"}</p>
            </div>

            <div class="user-voting-info">
                <div class="user-info">
                    <h3>{format!("🗳️ Votant: {}", user.username)}</h3>
                    <p>{format!("Réputation: {} points", user.reputation)}</p>
                </div>
                
                <div class="voting-power">
                    <h4>{"Votre pouvoir de vote"}</h4>
                    <div class="power-meter">
                        <div class="power-bar" style={format!("width: {}%", (user.reputation as f32 / 200.0 * 100.0).min(100.0))}></div>
                    </div>
                    <small>{"Basé sur votre réputation (max: 200 points)"}</small>
                </div>
            </div>

            if let Some(message) = (*vote_message).as_ref() {
                <div class="vote-success-message">
                    <strong>{"✅ "}</strong> {message}
                </div>
            }

            <div class="voting-proposals">
                <h2>{"Propositions ouvertes au vote"}</h2>
                
                if voting_proposals.is_empty() {
                    <div class="no-proposals">
                        <p>{"Aucune proposition n'est actuellement ouverte au vote."}</p>
                        <button 
                            class="btn-secondary"
                            onclick={
                                let on_navigate = props.on_navigate.clone();
                                Callback::from(move |_| on_navigate.emit("proposals".to_string()))
                            }
                        >
                            {"✏️ Faire une proposition"}
                        </button>
                    </div>
                } else {
                    <div class="proposals-grid">
                        {for voting_proposals.iter().map(|proposal| {
                            let total_votes = proposal.votes_for + proposal.votes_against + proposal.votes_abstain;
                            let participation_rate = if proposal.total_eligible_voters > 0 {
                                (total_votes as f32 / proposal.total_eligible_voters as f32 * 100.0)
                            } else {
                                0.0
                            };
                            
                            let time_left = proposal.voting_deadline.signed_duration_since(Utc::now());
                            let days_left = time_left.num_days();
                            let hours_left = time_left.num_hours() % 24;
                            
                            let proposal_click = {
                                let proposal = proposal.clone();
                                let on_select = on_proposal_select.clone();
                                Callback::from(move |_| on_select.emit(proposal.clone()))
                            };
                            
                            html! {
                                <div class="voting-proposal-card">
                                    <div class="proposal-header" onclick={proposal_click}>
                                        <h3 class="proposal-title">{&proposal.law.title}</h3>
                                        <div class="proposal-badges">
                                            <span class={format!("badge badge-{}", proposal.law.status.to_string().to_lowercase())}>
                                                {proposal.law.status.to_string()}
                                            </span>
                                            <span class="badge badge-category">
                                                {proposal.law.category.to_string()}
                                            </span>
                                        </div>
                                    </div>
                                    
                                    <div class="proposal-content">
                                        <p class="proposal-description">
                                            {if proposal.law.content.len() > 150 {
                                                format!("{}...", &proposal.law.content[..150])
                                            } else {
                                                proposal.law.content.clone()
                                            }}
                                        </p>
                                    </div>
                                    
                                    <div class="voting-stats">
                                        <div class="vote-bars">
                                            <div class="vote-bar-container">
                                                <div class="vote-bar vote-for" style={format!("width: {}%", if total_votes > 0 { proposal.votes_for as f32 / total_votes as f32 * 100.0 } else { 0.0 })}></div>
                                                <span class="vote-count">{"POUR: "}{proposal.votes_for}</span>
                                            </div>
                                            <div class="vote-bar-container">
                                                <div class="vote-bar vote-against" style={format!("width: {}%", if total_votes > 0 { proposal.votes_against as f32 / total_votes as f32 * 100.0 } else { 0.0 })}></div>
                                                <span class="vote-count">{"CONTRE: "}{proposal.votes_against}</span>
                                            </div>
                                            <div class="vote-bar-container">
                                                <div class="vote-bar vote-abstain" style={format!("width: {}%", if total_votes > 0 { proposal.votes_abstain as f32 / total_votes as f32 * 100.0 } else { 0.0 })}></div>
                                                <span class="vote-count">{"ABSTENTION: "}{proposal.votes_abstain}</span>
                                            </div>
                                        </div>
                                        
                                        <div class="participation-info">
                                            <div class="participation-rate">
                                                <strong>{format!("Participation: {:.1}%", participation_rate)}</strong>
                                                <small>{format!("({} / {} votants)", total_votes, proposal.total_eligible_voters)}</small>
                                            </div>
                                            
                                            <div class="time-remaining">
                                                if days_left > 0 {
                                                    <span class="time-left">{format!("⏰ {} jours {} heures restantes", days_left, hours_left)}</span>
                                                } else if hours_left > 0 {
                                                    <span class="time-left urgent">{format!("⚠️ {} heures restantes", hours_left)}</span>
                                                } else {
                                                    <span class="time-left expired">{"🔴 Vote terminé"}</span>
                                                }
                                            </div>
                                        </div>
                                    </div>
                                    
                                    <div class="voting-actions">
                                        if let Some(user_vote) = &proposal.user_vote {
                                            <div class="current-vote">
                                                <span>{"Votre vote: "}</span>
                                                <strong class={format!("vote-type-{}", 
                                                    match user_vote {
                                                        VoteType::For => "for",
                                                        VoteType::Against => "against",
                                                        VoteType::Abstain => "abstain",
                                                    }
                                                )}>
                                                    {match user_vote {
                                                        VoteType::For => "POUR",
                                                        VoteType::Against => "CONTRE",
                                                        VoteType::Abstain => "ABSTENTION",
                                                    }}
                                                </strong>
                                            </div>
                                        }
                                        
                                        if time_left.num_seconds() > 0 {
                                            <div class="vote-buttons">
                                                <button 
                                                    class={if matches!(proposal.user_vote, Some(VoteType::For)) { "vote-btn vote-for active" } else { "vote-btn vote-for" }}
                                                    onclick={
                                                        let on_vote = on_vote.clone();
                                                        let proposal_id = proposal.law.id;
                                                        Callback::from(move |_| on_vote.emit((proposal_id, VoteType::For)))
                                                    }
                                                    disabled={*is_voting}
                                                >
                                                    {"👍 POUR"}
                                                </button>
                                                <button 
                                                    class={if matches!(proposal.user_vote, Some(VoteType::Against)) { "vote-btn vote-against active" } else { "vote-btn vote-against" }}
                                                    onclick={
                                                        let on_vote = on_vote.clone();
                                                        let proposal_id = proposal.law.id;
                                                        Callback::from(move |_| on_vote.emit((proposal_id, VoteType::Against)))
                                                    }
                                                    disabled={*is_voting}
                                                >
                                                    {"👎 CONTRE"}
                                                </button>
                                                <button 
                                                    class={if matches!(proposal.user_vote, Some(VoteType::Abstain)) { "vote-btn vote-abstain active" } else { "vote-btn vote-abstain" }}
                                                    onclick={
                                                        let on_vote = on_vote.clone();
                                                        let proposal_id = proposal.law.id;
                                                        Callback::from(move |_| on_vote.emit((proposal_id, VoteType::Abstain)))
                                                    }
                                                    disabled={*is_voting}
                                                >
                                                    {"🤷 ABSTENTION"}
                                                </button>
                                            </div>
                                        }
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                }
            </div>

            // Détail de la proposition sélectionnée
            if let Some(proposal) = (*selected_proposal).as_ref() {
                <div class="proposal-detail-overlay" onclick={on_close_detail.clone()}>
                    <div class="proposal-detail-modal" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                        <div class="proposal-detail-header">
                            <h2>{&proposal.law.title}</h2>
                            <button class="close-btn" onclick={on_close_detail}>{"×"}</button>
                        </div>
                        
                        <div class="proposal-detail-content">
                            <div class="proposal-full-text">
                                <h4>{"Texte complet de la proposition:"}</h4>
                                <div class="proposal-text">
                                    {proposal.law.content.split('\n').map(|paragraph| {
                                        html! { <p>{paragraph}</p> }
                                    }).collect::<Html>()}
                                </div>
                            </div>
                            
                            <div class="voting-details">
                                <h4>{"Détails du vote:"}</h4>
                                <div class="vote-summary">
                                    <div class="vote-option">
                                        <span class="vote-label pour">{"POUR"}</span>
                                        <span class="vote-count">{proposal.votes_for}</span>
                                        <div class="vote-percentage">
                                            {format!("({:.1}%)", 
                                                if proposal.votes_for + proposal.votes_against + proposal.votes_abstain > 0 {
                                                    proposal.votes_for as f32 / (proposal.votes_for + proposal.votes_against + proposal.votes_abstain) as f32 * 100.0
                                                } else { 0.0 }
                                            )}
                                        </div>
                                    </div>
                                    <div class="vote-option">
                                        <span class="vote-label contre">{"CONTRE"}</span>
                                        <span class="vote-count">{proposal.votes_against}</span>
                                        <div class="vote-percentage">
                                            {format!("({:.1}%)",
                                                if proposal.votes_for + proposal.votes_against + proposal.votes_abstain > 0 {
                                                    proposal.votes_against as f32 / (proposal.votes_for + proposal.votes_against + proposal.votes_abstain) as f32 * 100.0
                                                } else { 0.0 }
                                            )}
                                        </div>
                                    </div>
                                    <div class="vote-option">
                                        <span class="vote-label abstention">{"ABSTENTION"}</span>
                                        <span class="vote-count">{proposal.votes_abstain}</span>
                                        <div class="vote-percentage">
                                            {format!("({:.1}%)",
                                                if proposal.votes_for + proposal.votes_against + proposal.votes_abstain > 0 {
                                                    proposal.votes_abstain as f32 / (proposal.votes_for + proposal.votes_against + proposal.votes_abstain) as f32 * 100.0
                                                } else { 0.0 }
                                            )}
                                        </div>
                                    </div>
                                </div>
                                
                                <div class="vote-metadata">
                                    <p><strong>{"Échéance du vote:"}</strong> {proposal.voting_deadline.format("%d/%m/%Y à %H:%M").to_string()}</p>
                                    <p><strong>{"Votants éligibles:"}</strong> {proposal.total_eligible_voters}</p>
                                    <p><strong>{"Auteur:"}</strong> {&proposal.law.author}</p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            }
        </div>
    }
}

// Données mock pour le développement
fn get_mock_voting_proposals() -> Vec<VotingProposal> {
    vec![
        VotingProposal {
            law: common::Law {
                id: Uuid::new_v4(),
                title: "Proposition: Semaine de travail de 4 jours".to_string(),
                content: "Cette proposition vise à réduire la semaine de travail standard à 4 jours (32 heures) tout en maintenant les salaires actuels.\n\nArticle 1: La durée légale du travail est fixée à 32 heures par semaine.\n\nArticle 2: Les entreprises de plus de 50 employés ont 2 ans pour s'adapter.\n\nArticle 3: Les secteurs essentiels (santé, sécurité) peuvent demander des dérogations.".to_string(),
                category: LawCategory::Economic,
                status: LawStatus::UnderReview,
                author: "Coalition pour l'équilibre vie-travail".to_string(),
                created_at: Utc::now() - chrono::Duration::days(15),
                updated_at: None,
                version: 1,
            },
            votes_for: 847,
            votes_against: 312,
            votes_abstain: 89,
            total_eligible_voters: 1500,
            voting_deadline: Utc::now() + chrono::Duration::days(5),
            user_vote: None,
        },
        VotingProposal {
            law: common::Law {
                id: Uuid::new_v4(),
                title: "Amendement: Augmentation du salaire minimum".to_string(),
                content: "Amendement à la loi sur le salaire minimum pour l'augmenter de 15% sur 2 ans.\n\nArticle 1: Le salaire minimum horaire est augmenté de 7.5% dès l'adoption de cet amendement.\n\nArticle 2: Une seconde augmentation de 7.5% sera appliquée 12 mois après.\n\nArticle 3: Les petites entreprises (moins de 10 employés) bénéficient d'aides fiscales pour compenser.".to_string(),
                category: LawCategory::Economic,
                status: LawStatus::Proposed,
                author: "Syndicat des travailleurs unis".to_string(),
                created_at: Utc::now() - chrono::Duration::days(8),
                updated_at: None,
                version: 1,
            },
            votes_for: 1023,
            votes_against: 578,
            votes_abstain: 156,
            total_eligible_voters: 1900,
            voting_deadline: Utc::now() + chrono::Duration::days(12),
            user_vote: Some(VoteType::For),
        },
        VotingProposal {
            law: common::Law {
                id: Uuid::new_v4(),
                title: "Loi sur la protection des lanceurs d'alerte".to_string(),
                content: "Cette loi vise à protéger les personnes qui signalent des crimes ou des violations dans l'intérêt public.\n\nArticle 1: Toute personne peut signaler des violations de loi sans crainte de représailles.\n\nArticle 2: Les employeurs ne peuvent licencier ou sanctionner les lanceurs d'alerte.\n\nArticle 3: Un fonds de protection juridique est créé pour assister les lanceurs d'alerte.".to_string(),
                category: LawCategory::Civil,
                status: LawStatus::Proposed,
                author: "ONG Transparence et Justice".to_string(),
                created_at: Utc::now() - chrono::Duration::days(3),
                updated_at: None,
                version: 1,
            },
            votes_for: 234,
            votes_against: 89,
            votes_abstain: 45,
            total_eligible_voters: 1200,
            voting_deadline: Utc::now() + chrono::Duration::hours(18),
            user_vote: None,
        },
    ]
}