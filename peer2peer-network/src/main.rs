use libp2p::{
    core::upgrade,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    futures::StreamExt,
    identity,
    mdns::{Mdns, MdnsEvent},
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{NetworkBehaviourEventProcess, Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    NetworkBehaviour, PeerId, Transport,
};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::{fs, io::AsyncBufReadExt, sync::mpsc};

const STORAGE_FILE_PATH: &str = "./users.json";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
type Users = Vec<User>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("users"));

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: usize,
    name: String,
    pronouns: String,
    phone_number: String,
    public: bool,
}

#[derive(Debug, Serialize, Deserialize)]
enum ListMode {
    ALL,
    One(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct ListRequest {
    mode: ListMode,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListResponse {
    mode: ListMode,
    data: Users,
    receiver: String,
}

enum EventType {
    Response(ListResponse),
    Input(String),
}

#[derive(NetworkBehaviour)]
struct UserBehaviour {
    floodsub: Floodsub,
    mdns: Mdns,
    #[behaviour(ignore)]
    response_sender: mpsc::UnboundedSender<ListResponse>,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for UserBehaviour {
    fn inject_event(&mut self, event: FloodsubEvent) {
        match event {
            FloodsubEvent::Message(msg) => {
                if let Ok(resp) = serde_json::from_slice::<ListResponse>(&msg.data) {
                    if resp.receiver == PEER_ID.to_string() {
                        info!("Response from {}:", msg.source);
                        resp.data.iter().for_each(|r| info!("{:?}", r));
                    }
                } else if let Ok(req) = serde_json::from_slice::<ListRequest>(&msg.data) {
                    match req.mode {
                        ListMode::ALL => {
                            info!("Received ALL req: {:?} from {:?}", req, msg.source);
                            respond_with_public_users(
                                self.response_sender.clone(),
                                msg.source.to_string(),
                            );
                        }
                        ListMode::One(ref peer_id) => {
                            if peer_id == &PEER_ID.to_string() {
                                info!("Received req: {:?} from {:?}", req, msg.source);
                                respond_with_public_users(
                                    self.response_sender.clone(),
                                    msg.source.to_string(),
                                );
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

fn respond_with_public_users(sender: mpsc::UnboundedSender<ListResponse>, receiver: String) {
    tokio::spawn(async move {
        match read_local_users().await {
            Ok(users) => {
                let resp = ListResponse {
                    mode: ListMode::ALL,
                    receiver,
                    data: users.into_iter().filter(|r| r.public).collect(),
                };
                if let Err(e) = sender.send(resp) {
                    error!("error sending response via channel, {}", e);
                }
            }
            Err(e) => error!("error fetching local users to answer ALL request, {}", e),
        }
    });
}

impl NetworkBehaviourEventProcess<MdnsEvent> for UserBehaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_list) => {
                for (peer, _addr) in discovered_list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

async fn create_new_user(name: &str, pronouns: &str, phone_number: &str) -> Result<()> {
    let mut local_users = read_local_users().await?;
    let new_id = match local_users.iter().max_by_key(|r| r.id) {
        Some(v) => v.id + 1,
        None => 0,
    };
    local_users.push(User {
        id: new_id,
        name: name.to_owned(),
        pronouns: pronouns.to_owned(),
        phone_number: phone_number.to_owned(),
        public: false,
    });
    write_local_users(&local_users).await?;

    info!("Created user:");
    info!("Name: {}", name);
    info!("Ingredients: {}", pronouns);
    info!("Instructions:: {}", phone_number);

    Ok(())
}

async fn publish_user(id: usize) -> Result<()> {
    let mut local_users = read_local_users().await?;
    local_users
        .iter_mut()
        .filter(|r| r.id == id)
        .for_each(|r| r.public = true);
    write_local_users(&local_users).await?;
    Ok(())
}

async fn read_local_users() -> Result<Users> {
    let content = fs::read(STORAGE_FILE_PATH).await?;
    let result = serde_json::from_slice(&content)?;
    Ok(result)
}

async fn write_local_users(users: &Users) -> Result<()> {
    let json = serde_json::to_string(&users)?;
    fs::write(STORAGE_FILE_PATH, &json).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    info!("Peer Id: {}", PEER_ID.clone());
    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&KEYS)
        .expect("can create auth keys");

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated()) // XX Handshake pattern, IX exists as well and IK - only XX currently provides interop with other libp2p impls
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let mut behaviour = UserBehaviour {
        floodsub: Floodsub::new(PEER_ID.clone()),
        mdns: Mdns::new(Default::default())
            .await
            .expect("can create mdns"),
        response_sender,
    };

    behaviour.floodsub.subscribe(TOPIC.clone());

    let mut swarm = SwarmBuilder::new(transp, behaviour, PEER_ID.clone())
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
    .expect("swarm can be started");

    loop {
        let evt = {
            tokio::select! {
                line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                response = response_rcv.recv() => Some(EventType::Response(response.expect("response exists"))),
                event = swarm.select_next_some() => {
                    info!("Unhandled Swarm Event: {:?}", event);
                    None
                },
            }
        };

        if let Some(event) = evt {
            match event {
                EventType::Response(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(TOPIC.clone(), json.as_bytes());
                }
                EventType::Input(line) => match line.as_str() {
                    "ls p" => handle_list_peers(&mut swarm).await,
                    cmd if cmd.starts_with("ls r") => handle_list_users(cmd, &mut swarm).await,
                    cmd if cmd.starts_with("create r") => handle_create_user(cmd).await,
                    cmd if cmd.starts_with("publish r") => handle_publish_user(cmd).await,
                    _ => error!("unknown command"),
                },
            }
        }
    }
}

async fn handle_list_peers(swarm: &mut Swarm<UserBehaviour>) {
    info!("Discovered Peers:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().for_each(|p| info!("{}", p));
}

async fn handle_list_users(cmd: &str, swarm: &mut Swarm<UserBehaviour>) {
    let rest = cmd.strip_prefix("ls r ");
    match rest {
        Some("all") => {
            let req = ListRequest {
                mode: ListMode::ALL,
            };
            let json = serde_json::to_string(&req).expect("can jsonify request");
            swarm
                .behaviour_mut()
                .floodsub
                .publish(TOPIC.clone(), json.as_bytes());
        }
        Some(users_peer_id) => {
            let req = ListRequest {
                mode: ListMode::One(users_peer_id.to_owned()),
            };
            let json = serde_json::to_string(&req).expect("can jsonify request");
            swarm
                .behaviour_mut()
                .floodsub
                .publish(TOPIC.clone(), json.as_bytes());
        }
        None => {
            match read_local_users().await {
                Ok(v) => {
                    info!("Local Users ({})", v.len());
                    v.iter().for_each(|r| info!("{:?}", r));
                }
                Err(e) => error!("error fetching local users: {}", e),
            };
        }
    };
}

async fn handle_create_user(cmd: &str) {
    if let Some(rest) = cmd.strip_prefix("create r") {
        let elements: Vec<&str> = rest.split("|").collect();
        if elements.len() < 3 {
            info!("too few arguments - Format: name|pronouns|phone_number");
        } else {
            let name = elements.get(0).expect("name is there");
            let pronouns = elements.get(1).expect("pronouns is there");
            let phone_number = elements.get(2).expect("phone_number is there");
            if let Err(e) = create_new_user(name, pronouns, phone_number).await {
                error!("error creating user: {}", e);
            };
        }
    }
}

async fn handle_publish_user(cmd: &str) {
    if let Some(rest) = cmd.strip_prefix("publish r") {
        match rest.trim().parse::<usize>() {
            Ok(id) => {
                if let Err(e) = publish_user(id).await {
                    info!("error publishing user with id {}, {}", id, e)
                } else {
                    info!("Published User with id: {}", id);
                }
            }
            Err(e) => error!("invalid id: {}, {}", rest.trim(), e),
        };
    }
}
