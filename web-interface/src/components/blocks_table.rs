use yew::prelude::*;
use crate::types::Block;

#[derive(Properties, PartialEq)]
pub struct BlocksTableProps {
    pub blocks: Vec<Block>,
}

#[function_component(BlocksTable)]
pub fn blocks_table(props: &BlocksTableProps) -> Html {
    html! {
        <table class="table">
            <thead>
                <tr>
                    <th>{"🔢 Numéro"}</th>
                    <th>{"🔗 Hash"}</th>
                    <th>{"📝 Transactions"}</th>
                    <th>{"⏰ Timestamp"}</th>
                    <th>{"⛏️ Mineur"}</th>
                </tr>
            </thead>
            <tbody>
                {
                    if props.blocks.is_empty() {
                        html! {
                            <tr>
                                <td colspan="5" style="text-align: center; opacity: 0.7;">
                                    {"Aucun bloc disponible"}
                                </td>
                            </tr>
                        }
                    } else {
                        props.blocks.iter().map(|block| {
                            let hash_short = if block.hash.len() > 16 {
                                format!("{}...{}", &block.hash[0..8], &block.hash[block.hash.len()-8..])
                            } else {
                                block.hash.clone()
                            };
                            
                            html! {
                                <tr key={block.number.to_string()}>
                                    <td>{block.number}</td>
                                    <td title={block.hash.clone()}>{hash_short}</td>
                                    <td>{block.transaction_count}</td>
                                    <td>{block.timestamp.format("%H:%M:%S").to_string()}</td>
                                    <td>{block.miner.as_ref().unwrap_or(&"Inconnu".to_string())}</td>
                                </tr>
                            }
                        }).collect::<Html>()
                    }
                }
            </tbody>
        </table>
    }
}