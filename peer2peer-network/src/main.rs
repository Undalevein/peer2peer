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
use slint::Model;

const STORAGE_FILE_PATH: &str = "./users.json";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
type Users = Vec<User>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("users"));

slint::slint! {
    import { Button, VerticalBox, GroupBox, TextEdit, HorizontalBox, LineEdit , ScrollView  } from "std-widgets.slint";

    component UserContact inherits Rectangle {
        in property <int> id;
        in property <string> name;
        in property <string> pronouns;
        in property <string> phone_number;
        in property <string> about_me;
        in property <string> messages;
        in property <int> messages_length;
        in property <bool> public;

        background: #005BEE;
        HorizontalBox {
            VerticalBox {
                alignment: start;
                Text {
                    text: name;
                }
                Text {
                    text: pronouns;
                }
                Text {
                    text: phone_number;
                }
                Text {
                    text: "User ID: " + id;
                }
            }
            VerticalBox {
                Text {
                    text: about_me;
                }
            }
        }

    }

    struct UserData {
        id: int,
        name: string,
        pronouns: string,
        phone_number: string,
        about_me: string,
        messages: string,
        messages_length: int,
        public: bool,
    }

    export component MainWindow inherits Window {
        callback add_user_button_hit(string);
        callback edit_user_button_hit(string);
        callback message_user_button_hit(string);

        width: 1000px;
        height: 700px;

        in property <[UserData]> user_list: [];

        GroupBox {
            title: "Peer2Peer Network - Texting Program";
            // Add someone to Contact List
            VerticalBox {
                Text {
                    text: "Add a new user to your contacts.";
                    height: 50px;
                }
                HorizontalBox {
                    name_input := LineEdit {
                        font-size: 14px;
                        height: 30px;
                        placeholder-text: "Name";
                        edited => {
                            add_button.text = "Add User";
                        }
                    }
                    pronouns_input := LineEdit {
                        font-size: 14px;
                        height: 30px;
                        placeholder-text: "Pronouns";
                        edited => {
                            add_button.text = "Add User";
                        }
                    }
                    phone_input := LineEdit {
                        font-size: 14px;
                        height: 30px;
                        placeholder-text: "Phone Number";
                        edited => {
                            add_button.text = "Add User";
                        }
                    }
                    add_button := Button {
                        text: "Add User";
                        height: 30px;
                        clicked => {
                            if (name_input.text != "" && pronouns_input.text != "" && phone_input.text != ""){
                                self.text = "ADDED!"; 
                                root.add_user_button_hit("create u " + name_input.text + "|" +  pronouns_input.text + "|" + phone_input.text);
                                name_input.text = "";
                                pronouns_input.text = "";
                                phone_input.text = "";
                            }
                        }
                    }
                }
                // Edit someone in Contact List
                HorizontalBox {
                    id_input := LineEdit {
                        font-size: 14px;
                        height: 30px;
                        placeholder-text: "ID";
                        edited => {
                            edit_button.text = "Edit User";
                        }
                    }
                    change_input := LineEdit {
                        font-size: 14px;
                        height: 30px;
                        placeholder-text: "[name/phone/about]";
                        edited => {
                            edit_button.text = "Edit User";
                        }
                    }
                    replacement_input := LineEdit {
                        font-size: 14px;
                        height: 30px;
                        placeholder-text: "Replacement";
                        edited => {
                            edit_button.text = "Edit User";
                        }
                    }
                    edit_button := Button {
                        text: "Edit User";
                        height: 30px;
                        clicked => {
                            if (id_input.text != "" && (change_input.text == "name" || change_input.text == "phone" || change_input.text == "about") && replacement_input.text != ""){
                                self.text = "EDITED!"; 
                                root.edit_user_button_hit("edit u " + id_input.text + "|" +  change_input.text + "|" + replacement_input.text);
                                id_input.text = "";
                                change_input.text = "";
                                replacement_input.text = "";
                            }
                        }
                    }
                }
                // Message User
                Text {
                    text: "Messages";
                    font-size: 30px;
                }
                ScrollView {
                    height: 200px;
                    viewport_height: 100px;
    
                    VerticalLayout{
                        alignment: start;
                        message := Text {
                            font-size: 14px;
                            text: "[Damian]: Hello!";
                        }
                    }
                    
                }
                message_input := TextEdit {
                    height: 50px;
                    text: "";
                    edited => {
                        send_button.text = "Send";
                    }
                }
                HorizontalBox {
                    name_to_message_input := LineEdit{
                        placeholder-text: "Enter Name to Send";
                        edited => {
                            send_button.text = "Send";
                        }
                    }
                    send_button := Button {
                        text: "Send";
                        height: 30px;
                        clicked => {
                            if (message_input.text != "" && name_to_message_input.text != "") {
                                self.text = "SENT!"; 
                                root.message_user_button_hit("message u " + name_to_message_input.text + "|" + message_input.text);
                                message_input.text = "";
                                name_to_message_input.text = "";
                            }
                        }
                    }

                }
            }
            // Contact List
            VerticalBox {
                Text {
                    text: "Contacts List";
                    color: white;
                    height: 20px;
                }
                contacts := ScrollView {
                    width: 300px;
                    height: 600px;
                    viewport_width: 300px;
                    viewport_height: 3000px;

                    for user[i] in user_list : UserContact {
                        x: 0;
                        y: i * 34px;

                        width: 290px;
                        height: 30px;
                        
                        id: user.id;
                        name: user.name;
                        pronouns: user.pronouns;
                        phone_number: user.phone_number;
                        about_me: user.about_me;
                        messages: user.messages;
                        messages_length: user.messages_length;
                        public: user.public;
                    }
                }
            }
        }
        
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info) // Include info level and higher
        .init();

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

    // Slint User Interface
    let ui = MainWindow::new().unwrap();
    let mut user_contact_gui: Vec<UserData> = ui.get_user_list().iter().collect();
    tokio::spawn(async move {
        let local_users = read_local_users().await.expect("REASON");
        for i in 0..local_users.len() {
            user_contact_gui.push(UserData {
                id: local_users[i].id as i32,
                name: local_users[i].name.to_string().into(),
                pronouns: local_users[i].pronouns.to_string().into(),
                phone_number: local_users[i].phone_number.to_string().into(),
                about_me: local_users[i].about_me.to_string().into(),
                messages: local_users[i].messages.to_string().into(),
                messages_length: local_users[i].messages_length as i32,
                public: local_users[i].public,
            });
        }
    });
    ui.on_add_user_button_hit(move |to_add| { 
        tokio::spawn(async move {
            info!("ADDED USER {}", to_add);
            handle_create_user(&to_add).await;
        });
    });
    ui.on_edit_user_button_hit(move |to_edit| {
        tokio::spawn(async move {
            info!("EDITED USER {}", to_edit);
            handle_edit_user(&to_edit).await;
        });
    });
    ui.on_message_user_button_hit(move |to_message| {
        tokio::spawn(async move {
            info!("MESSAGE USER {}", to_message);
            handle_message_user(&to_message).await;
        });
    });
    //  let user_contact_model = std::rc::Rc::new(slint::VecModel::from(user_contact_gui));
    //  ui.set_user_list(user_contact_model.into());

    ui.run().unwrap();


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
                    cmd if cmd.starts_with("ls u") => handle_list_users(cmd, &mut swarm).await,
                    cmd if cmd.starts_with("create u") => handle_create_user(cmd).await,
                    cmd if cmd.starts_with("publish u") => handle_publish_user(cmd).await,
                    cmd if cmd.starts_with("message u") => handle_message_user(cmd).await,
                    cmd if cmd.starts_with("edit u") => handle_edit_user(cmd).await,
                    cmd if cmd.starts_with("help") => handle_help().await,
                    _ => error!("unknown command"),
                },
            }
        }
    }
    
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: usize,
    name: String,
    pronouns: String,
    phone_number: String,
    about_me: String,
    messages: String,
    messages_length: u32,
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
                        resp.data.iter().for_each(|u| info!("{:?}", u));
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
                    data: users.into_iter().filter(|u| u.public).collect(),
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
    let new_id = match local_users.iter().max_by_key(|u| u.id) {
        Some(v) => v.id + 1,
        None => 0,
    };
    local_users.push(User {
        id: new_id,
        name: name.to_owned(),
        pronouns: pronouns.to_owned(),
        phone_number: phone_number.to_owned(),
        about_me: "".to_string().to_owned(),
        messages: "".to_string().to_owned(),
        messages_length: 0,
        public: false,
    });
    write_local_users(&local_users).await?;

    info!("Created user:");
    info!("Name: {}", name);
    info!("Pronouns: {}", pronouns);
    info!("Phone Number: {}", phone_number);

    Ok(())
}

async fn publish_user(id: usize) -> Result<()> {
    let mut local_users = read_local_users().await?;
    local_users
        .iter_mut()
        .filter(|u| u.id == id)
        .for_each(|u| u.public = true);
    write_local_users(&local_users).await?;
    Ok(())
}

async fn message_user(name: &str, message: &str) -> Result<()> {
    let mut local_users = read_local_users().await?;
    
    for i in 0..local_users.len(){
        if (local_users[i].name).eq(name) {
            local_users[i].messages += "{";
            local_users[i].messages += message;
            local_users[i].messages += "}; ";
            local_users[i].messages_length += 1;
            break;
        }
    } 
    write_local_users(&local_users).await?;
    Ok(())
}

async fn edit_user(id: usize, option_change: &str, replacement: &str) -> Result<()> {
    let mut local_users = read_local_users().await?;
    
    for i in 0..local_users.len(){
        if local_users[i].id == id {
            if option_change == "name"{
                local_users[i].name = replacement.to_string();
            } else if option_change == "phone" {
                local_users[i].phone_number = replacement.to_string();
            } else if option_change == "about" {
                local_users[i].about_me = replacement.to_string();
            }
            break;
        }
    } 
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
    let rest = cmd.strip_prefix("ls u ");
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
                    v.iter().for_each(|u| info!("{:?}", u));
                }
                Err(e) => error!("error fetching local users: {}", e),
            };
        }
    };
}

async fn handle_create_user(cmd: &str) {
    info!("Added {}", cmd);
    if let Some(rest) = cmd.strip_prefix("create u") {
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
    if let Some(rest) = cmd.strip_prefix("publish u") {
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

async fn handle_message_user(cmd: &str) {
    if let Some(rest) = cmd.strip_prefix("message u ") {
        let elements: Vec<&str> = rest.split("|").collect();
        if elements.len() < 2 {
            info!("too few arguments - Format: message u name|message");
        } else {
            let name = elements.get(0).expect("name is there");
            let message = elements.get(1).expect("message is there");
            if let Err(e) = message_user(name, &message).await {
                error!("user {} cannot be found: {}", name, e);
            }
        }
    }
}

async fn handle_edit_user(cmd: &str) {
    if let Some(rest) = cmd.strip_prefix("edit u "){
        let elements: Vec<&str> = rest.split("|").collect();
        if elements.len() < 3 {
            info!("too few arguments - Format: edit u id|[name/phone/about]|[element]");
        } else {
            // Checks if id is a valid unsigned integer.
            let id_string = elements.get(0).expect("id is there");
            let mut id = 0;
            match id_string.trim().parse::<usize>() {
                Ok(id_number) => {
                    id = id_number;
                }
                Err(e) => error!("invalid id: {}, {}", id_string.trim(), e),
            };

            // Checks if the option for what option to edit is valid.
            let argument = elements.get(1).expect("type is there");
            if argument != &"name" && argument != &"phone" && argument != &"about"{
                error!("invalid argument: {} is not name, phone, or about", argument)
            }
            
            // Gets the replacement
            let replacement = elements.get(2).expect("replacement is there");


            if let Err(e) = edit_user(id, &argument, &replacement).await {
                error!("invalid id: {} cannot be found: {}", id, e)
            }
            
        }
    }
}

async fn handle_help() {
    info!("ls p - list all peers");
    info!("ls u - list local users");
    info!("ls u all - list all public users from all known peers");
    info!("ls u {{peerId}} - list all public users from the given peer");
    info!("create u Name|Pronouns|Phone_Number - create a new user with the given data, the `|` are important as separators");
    info!("publish u {{userId}} - publish user with the given user ID");
    info!("message u Name|Message - message a user a message");
    info!("edit u ID|[name/phone/about] - edit a user profile by editing the user's name, phone number, or about me");
    info!("help - list the commands of the program");
}