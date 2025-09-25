use yew::prelude::*;
use gloo::timers::callback::Interval;
use crate::services::BlockchainService;
use crate::types::*;

use crate::components::{StatsCard, BlocksTable, TransactionsTable, PeersTable, ServersTable};

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    let blockchain_service = use_memo((), |_| {
        // Nouvelles variables build-time :
        // API_BASE_HOST (ex: http://localhost)
        // API_SERVER_PORTS (ex: 3000,3002,3004)
        let host = option_env!("API_BASE_HOST").unwrap_or("http://localhost");
        let ports_env = option_env!("API_SERVER_PORTS").unwrap_or("3000,3002,3004");
        let ports: Vec<u16> = ports_env
            .split(',')
            .filter_map(|p| p.trim().parse::<u16>().ok())
            .collect();
        BlockchainService::new_multi(host.to_string(), ports)
    });

    let stats = use_state(|| None::<BlockchainStats>);
    let blocks = use_state(|| Vec::<Block>::new());
    let transactions = use_state(|| Vec::<Transaction>::new());
    let peers = use_state(|| Vec::<NetworkPeer>::new());
    let servers = use_state(|| Vec::<ServerInfo>::new());
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);

    let refresh_data = {
        let stats = stats.clone();
        let blocks = blocks.clone();
        let transactions = transactions.clone();
        let peers = peers.clone();
        let servers = servers.clone();
        let loading = loading.clone();
        let error = error.clone();
        let service = blockchain_service.clone();

        Callback::from(move |_| {
            let stats = stats.clone();
            let blocks = blocks.clone();
            let transactions = transactions.clone();
            let peers = peers.clone();
            let servers = servers.clone();
            let loading = loading.clone();
            let error = error.clone();
            let service = (*service).clone();

            loading.set(true);
            error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                // Récupérer les statistiques
                match service.get_health().await {
                    Ok(_) => {
                        // Service disponible, récupérer les données
                        let mut has_error = false;

                        // Ne pas récupérer de statistiques fictives
                        stats.set(None);

                        // Récupérer les blocs
                        match service.get_latest_blocks(10).await {
                            Ok(latest_blocks) => blocks.set(latest_blocks),
                            Err(e) => {
                                error.set(Some(format!("Erreur blocs: {}", e)));
                                has_error = true;
                            }
                        }

                        // Ne pas récupérer de transactions (endpoint pas encore implémenté côté serveur)
                        transactions.set(Vec::new());

                        // Récupérer les pairs
                        match service.get_network_peers().await {
                            Ok(network_peers) => peers.set(network_peers),
                            Err(e) => {
                                error.set(Some(format!("Erreur peers: {}", e)));
                                has_error = true;
                            }
                        }

                        // Récupérer les serveurs actifs
                        match service.get_active_servers().await {
                            Ok(active_servers) => servers.set(active_servers),
                            Err(_) => {
                                // Si les serveurs ne sont pas disponibles, laisser la liste vide
                                servers.set(Vec::new());
                            }
                        }

                        if !has_error {
                            error.set(None);
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("Service blockchain indisponible: {}", e)));
                    }
                }

                loading.set(false);
            });
        })
    };

    // Charger les données au démarrage
    {
        let refresh_data = refresh_data.clone();
        use_effect_with((), move |_| {
            refresh_data.emit(());
            || ()
        });
    }

    // Actualisation automatique toutes les 30 secondes
    {
        let refresh_data = refresh_data.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(30_000, move || {
                refresh_data.emit(());
            });
            
            move || drop(interval)
        });
    }

    html! {
        <div class="dashboard">
            <div class="dashboard-controls">
                <button 
                    class="refresh-btn"
                    onclick={move |_| refresh_data.emit(())}
                    disabled={*loading}
                >
                    if *loading {
                        {"⟳ Actualisation..."}
                    } else {
                        {"🔄 Actualiser"}
                    }
                </button>
            </div>

            if let Some(err) = (*error).as_ref() {
                <div class="error-message">
                    <strong>{"⚠️ Erreur: "}</strong>
                    {err}
                </div>
            }

            // Statistiques - affichage simple du nombre de blocs réels
            <div class="dashboard-grid">
                <StatsCard 
                    title="📦 Blocs Récents"
                    value={blocks.len().to_string()}
                    subtitle="blocs disponibles"
                />
                <StatsCard 
                    title="🌐 Pairs Réseau"
                    value={peers.len().to_string()}
                    subtitle="pairs connectés"
                />
                <StatsCard 
                    title="🖥️ Serveurs"
                    value={servers.len().to_string()}
                    subtitle="serveurs actifs"
                />
            </div>

            // Tableaux de données
            <div class="tables-container">
                <div class="table-container">
                    <h3>{"📦 Blocs Récents"}</h3>
                    <BlocksTable blocks={(*blocks).clone()} />
                </div>

                <div class="table-container">
                    <h3>{"📝 Transactions Récentes"}</h3>
                    <TransactionsTable transactions={(*transactions).clone()} />
                </div>

                <div class="table-container">
                    <h3>{"🌐 Pairs du Réseau"}</h3>
                    <PeersTable peers={(*peers).clone()} />
                </div>

                <div class="table-container">
                    <h3>{"🖥️ Serveurs Actifs"}</h3>
                    <ServersTable servers={(*servers).clone()} />
                </div>
            </div>
        </div>
    }
}