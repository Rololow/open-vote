use yew::prelude::*;
use crate::types::{NetworkPeer, PeerStatus};

#[derive(Properties, PartialEq)]
pub struct PeersTableProps {
    pub peers: Vec<NetworkPeer>,
}

#[function_component(PeersTable)]
pub fn peers_table(props: &PeersTableProps) -> Html {
    html! {
        <table class="table">
            <thead>
                <tr>
                    <th>{"🆔 ID"}</th>
                    <th>{"🌐 Adresse"}</th>
                    <th>{"📊 Statut"}</th>
                    <th>{"📦 Hauteur"}</th>
                    <th>{"👁️ Dernière activité"}</th>
                </tr>
            </thead>
            <tbody>
                {
                    if props.peers.is_empty() {
                        html! {
                            <tr>
                                <td colspan="5" style="text-align: center; opacity: 0.7;">
                                    {"Aucun pair connecté"}
                                </td>
                            </tr>
                        }
                    } else {
                        props.peers.iter().map(|peer| {
                            let id_short = if peer.id.len() > 12 {
                                format!("{}...{}", &peer.id[0..6], &peer.id[peer.id.len()-6..])
                            } else {
                                peer.id.clone()
                            };

                            let (status_class, status_text) = match peer.status {
                                PeerStatus::Connected => ("status-success", "🟢 Connecté"),
                                PeerStatus::Syncing => ("status-pending", "🔄 Synchronisation"),
                                PeerStatus::Disconnected => ("status-error", "🔴 Déconnecté"),
                            };
                            
                            html! {
                                <tr key={peer.id.clone()}>
                                    <td title={peer.id.clone()}>{id_short}</td>
                                    <td>{&peer.address}</td>
                                    <td>
                                        <span class={format!("status-badge {}", status_class)}>
                                            {status_text}
                                        </span>
                                    </td>
                                    <td>{peer.block_height}</td>
                                    <td>{peer.last_seen.format("%H:%M:%S").to_string()}</td>
                                </tr>
                            }
                        }).collect::<Html>()
                    }
                }
            </tbody>
        </table>
    }
}