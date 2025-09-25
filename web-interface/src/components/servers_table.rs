use yew::prelude::*;
use crate::types::{ServerInfo, ServerStatus};

#[derive(Properties, PartialEq)]
pub struct ServersTableProps {
    pub servers: Vec<ServerInfo>,
}

#[function_component(ServersTable)]
pub fn servers_table(props: &ServersTableProps) -> Html {
    html! {
        <table class="table">
            <thead>
                <tr>
                    <th>{"🖥️ Serveur"}</th>
                    <th>{"🌐 IP:Port"}</th>
                    <th>{"📊 Statut"}</th>
                    <th>{"⏰ Uptime"}</th>
                    <th>{"💾 Version"}</th>
                    <th>{"🔌 Connexions"}</th>
                    <th>{"📈 CPU/RAM"}</th>
                </tr>
            </thead>
            <tbody>
                {
                    if props.servers.is_empty() {
                        html! {
                            <tr>
                                <td colspan="7" style="text-align: center; opacity: 0.7;">
                                    {"Aucun serveur disponible"}
                                </td>
                            </tr>
                        }
                    } else {
                        props.servers.iter().map(|server| {
                            let (status_class, status_text, status_icon) = match server.status {
                                ServerStatus::Online => ("status-success", "En ligne", "🟢"),
                                ServerStatus::Offline => ("status-error", "Hors ligne", "🔴"),
                                ServerStatus::Maintenance => ("status-pending", "Maintenance", "🟡"),
                                ServerStatus::Warning => ("status-warning", "Attention", "🟠"),
                            };

                            let uptime_text = format_uptime(server.uptime);
                            
                            let usage_text = match (server.cpu_usage, server.memory_usage) {
                                (Some(cpu), Some(mem)) => format!("{:.1}% / {:.1}%", cpu, mem),
                                _ => "—".to_string(), // Données non disponibles
                            };
                            
                            html! {
                                <tr key={format!("{}:{}", server.ip, server.port)}>
                                    <td>
                                        <strong>{&server.name}</strong>
                                    </td>
                                    <td>
                                        <code>{format!("{}:{}", server.ip, server.port)}</code>
                                    </td>
                                    <td>
                                        <span class={format!("status-badge {}", status_class)}>
                                            {status_icon}{" "}{status_text}
                                        </span>
                                    </td>
                                    <td>{uptime_text}</td>
                                    <td>
                                        <span class="version-badge">{&server.version}</span>
                                    </td>
                                    <td>{if server.connections == 0 { "—".to_string() } else { server.connections.to_string() }}</td>
                                    <td>{usage_text}</td>
                                </tr>
                            }
                        }).collect::<Html>()
                    }
                }
            </tbody>
        </table>
    }
}

fn format_uptime(seconds: u64) -> String {
    if seconds == 0 {
        return "—".to_string(); // Pas de données disponibles
    }
    
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    
    if hours > 0 {
        format!("{}h {:02}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {:02}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}