use yew::prelude::*;
use crate::types::{Transaction, TransactionStatus};

#[derive(Properties, PartialEq)]
pub struct TransactionsTableProps {
    pub transactions: Vec<Transaction>,
}

#[function_component(TransactionsTable)]
pub fn transactions_table(props: &TransactionsTableProps) -> Html {
    html! {
        <table class="table">
            <thead>
                <tr>
                    <th>{"🆔 ID"}</th>
                    <th>{"📋 Type"}</th>
                    <th>{"👤 Expéditeur"}</th>
                    <th>{"📊 Statut"}</th>
                    <th>{"📦 Bloc"}</th>
                    <th>{"⏰ Timestamp"}</th>
                </tr>
            </thead>
            <tbody>
                {
                    if props.transactions.is_empty() {
                        html! {
                            <tr>
                                <td colspan="6" style="text-align: center; opacity: 0.7;">
                                    {"Aucune transaction disponible"}
                                </td>
                            </tr>
                        }
                    } else {
                        props.transactions.iter().map(|tx| {
                            let id_short = if tx.id.len() > 12 {
                                format!("{}...{}", &tx.id[0..6], &tx.id[tx.id.len()-6..])
                            } else {
                                tx.id.clone()
                            };
                            
                            let sender_short = if tx.sender.len() > 12 {
                                format!("{}...{}", &tx.sender[0..6], &tx.sender[tx.sender.len()-6..])
                            } else {
                                tx.sender.clone()
                            };

                            let (status_class, status_text) = match tx.status {
                                TransactionStatus::Confirmed => ("status-success", "✅ Confirmée"),
                                TransactionStatus::Pending => ("status-pending", "⏳ En attente"),
                                TransactionStatus::Failed => ("status-error", "❌ Échouée"),
                            };
                            
                            html! {
                                <tr key={tx.id.clone()}>
                                    <td title={tx.id.clone()}>{id_short}</td>
                                    <td>{&tx.transaction_type}</td>
                                    <td title={tx.sender.clone()}>{sender_short}</td>
                                    <td>
                                        <span class={format!("status-badge {}", status_class)}>
                                            {status_text}
                                        </span>
                                    </td>
                                    <td>
                                        {
                                            if let Some(block_num) = tx.block_number {
                                                block_num.to_string()
                                            } else {
                                                "-".to_string()
                                            }
                                        }
                                    </td>
                                    <td>{tx.timestamp.format("%H:%M:%S").to_string()}</td>
                                </tr>
                            }
                        }).collect::<Html>()
                    }
                }
            </tbody>
        </table>
    }
}