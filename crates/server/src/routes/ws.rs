use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::Serialize;
use tracing::{debug, info};

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SpectatorMsg<'a> {
    Snapshot {
        tick: u64,
        clock: &'a world::WorldClock,
        grid_width: u16,
        grid_height: u16,
        tiles: Vec<TileMsg>,
        agents: Vec<crate::state::SpectatorAgent>,
        entities: Vec<crate::state::SpectatorEntity>,
    },
    Tick {
        view: &'a crate::state::SpectatorView,
    },
}

#[derive(Serialize)]
struct TileMsg {
    pos: world::TileCoord,
    kind: world::TileKind,
    biome: world::Biome,
}

fn serde_kind<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|j| j.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".into())
}

pub async fn spectator_ws(ws: WebSocketUpgrade, State(s): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle(socket, s))
}

async fn handle(socket: WebSocket, s: AppState) {
    let (mut tx, mut rx) = socket.split();
    info!("spectator connected");

    {
        let w = s.world.lock().await;
        let tiles: Vec<TileMsg> = w
            .grid
            .iter()
            .map(|(pos, t)| TileMsg {
                pos,
                kind: t.kind,
                biome: t.biome,
            })
            .collect();
        let agents: Vec<crate::state::SpectatorAgent> = w
            .agents
            .values()
            .map(|a| crate::state::SpectatorAgent {
                id: a.id.clone(),
                name: a.name.clone(),
                pos: a.pos,
                hp: a.status.hp,
                hunger: a.status.hunger,
                stamina: a.status.stamina,
                state: match a.state {
                    world::AgentState::Alive => "alive".into(),
                    world::AgentState::Dying { .. } => "dying".into(),
                    world::AgentState::Meditating { .. } => "meditating".into(),
                },
                inventory: a.inventory.slots.clone(),
            })
            .collect();
        let mut entities = Vec::with_capacity(w.entities.len() + w.buildings.len());
        for (pos, e) in &w.entities {
            match e {
                world::Entity::Plant { plant } => entities.push(crate::state::SpectatorEntity {
                    pos: *pos,
                    kind: format!("plant:{}", serde_kind(&plant.kind)),
                    label: None,
                    id: None,
                }),
                world::Entity::ItemDrop { stack, .. } => entities.push(crate::state::SpectatorEntity {
                    pos: *pos,
                    kind: format!("drop:{}", serde_kind(&stack.item)),
                    label: Some(format!("×{}", stack.n)),
                    id: None,
                }),
            }
        }
        for (pos, b) in &w.buildings {
            entities.push(crate::state::SpectatorEntity {
                pos: *pos,
                kind: format!("building:{}", serde_kind(&b.kind)),
                label: None,
                id: None,
            });
        }
        for c in w.creatures.values() {
            entities.push(crate::state::SpectatorEntity {
                pos: c.pos,
                kind: format!("creature:{}", serde_kind(&c.kind)),
                label: Some(format!("{}/{}", c.hp, c.kind.max_hp())),
                id: Some(c.id),
            });
        }
        let msg = SpectatorMsg::Snapshot {
            tick: w.clock.tick,
            clock: &w.clock,
            grid_width: w.grid.width,
            grid_height: w.grid.height,
            tiles,
            agents,
            entities,
        };
        let json = serde_json::to_string(&msg).unwrap();
        if tx.send(Message::Text(json)).await.is_err() {
            return;
        }
    }

    let mut frames = s.frames_tx.subscribe();
    let send_loop = async {
        while let Ok(f) = frames.recv().await {
            let msg = SpectatorMsg::Tick {
                view: &f.spectator_view,
            };
            let json = serde_json::to_string(&msg).unwrap();
            if tx.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    };
    let recv_loop = async {
        while let Some(Ok(m)) = rx.next().await {
            debug!(?m, "spectator msg");
            if matches!(m, Message::Close(_)) {
                break;
            }
        }
    };
    tokio::select! { _ = send_loop => {}, _ = recv_loop => {} }
    info!("spectator disconnected");
}
