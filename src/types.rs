use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use eframe::egui::{self, Color32};
use serde_json::Value;
use crate::avatar_3d;
use crate::video_preview;

#[derive(Clone, Debug)]
pub struct Settings {
    pub dark: bool,
    pub target_fps: Option<u32>,
    pub auto_close_roblox: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dark: false,
            target_fps: None,
            auto_close_roblox: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Game {
    pub universe_id: u64,
    pub place_id: u64,
    pub name: String,
    pub creator: String,
    pub creator_verified: bool,
    pub playing: u64,
    pub up: u64,
    pub down: u64,
    pub description: String,
}

impl Game {
    pub fn rating(&self) -> String {
        let total = self.up + self.down;
        if total == 0 {
            "--".to_string()
        } else {
            format!("{}%", self.up * 100 / total)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Sort {
    pub title: String,
    pub games: Vec<Game>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameTab {
    About,
    Store,
    Servers,
}

impl GameTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::About => "About",
            Self::Store => "Store",
            Self::Servers => "Servers",
        }
    }
}

#[derive(Clone, Debug)]
pub struct GamePassItem {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub price: Option<u64>,
    pub is_for_sale: bool,
    pub is_owned: bool,
    pub thumb: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ServerSort {
    #[default]
    Default,
    LowestPing,
    FewestPlayers,
    MostPlayers,
    HighestFps,
}

impl ServerSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::LowestPing => "Lowest Ping",
            Self::FewestPlayers => "Fewest Players",
            Self::MostPlayers => "Most Players",
            Self::HighestFps => "Highest FPS",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ServerItem {
    pub id: String,
    pub playing: u64,
    pub max_players: u64,
    pub ping: Option<u64>,
    pub fps: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct GameExtra {
    pub universe_id: u64,
    pub verified_creator: bool,
    pub favorites: u64,
    pub visits: u64,
    pub max_players: u64,
    pub created: String,
    pub updated: String,
    pub genre: String,
    pub subgenre: String,
    pub maturity_short: String,
    pub maturity_detail: String,
    pub voice_chat: Option<bool>,
    pub camera: Option<bool>,
    pub is_favorited_api: Option<bool>,
    pub can_vote: bool,
    pub user_vote: Option<bool>,
    pub media: Vec<String>,
    pub preview_video_id: Option<u64>,
    pub passes: Vec<GamePassItem>,
    pub passes_loaded: bool,
    pub servers: Vec<ServerItem>,
    pub servers_cursor: Option<String>,
    pub servers_loaded: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub display: String,
    pub headshot: String,
    pub presence: u8,
    pub location: String,
    pub place_id: Option<u64>,
    pub verified: bool,
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub user: User,
    pub friends: u64,
    pub avatar: String,
}

#[derive(Clone, Debug)]
pub struct AvatarAsset {
    pub id: u64,
    pub name: String,
    pub asset_type_id: u64,
    pub asset_type_name: String,
    pub thumb: String,
    pub meta: Option<Value>,
    pub creator_name: String,
    pub creator_verified: bool,
    pub price: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvatarScales {
    pub height: f32,
    pub width: f32,
    pub head: f32,
    pub depth: f32,
    pub proportion: f32,
    pub body_type: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyColors {
    pub head: u64,
    pub torso: u64,
    pub right_arm: u64,
    pub left_arm: u64,
    pub right_leg: u64,
    pub left_leg: u64,
}

#[derive(Clone, Debug)]
pub struct OutfitItem {
    pub id: u64,
    pub name: String,
    pub thumb: String,
}

#[derive(Clone, Debug)]
pub struct PaletteColor {
    pub id: u64,
    pub name: String,
    pub color: Color32,
}

#[derive(Clone, Debug)]
pub struct AvatarState {
    pub avatar_type: String,
    pub scales: AvatarScales,
    pub colors: BodyColors,
    pub wearing: Vec<AvatarAsset>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AvatarTab {
    Wearing,
    Outfits,
    Accessories,
    Hair,
    Shirts,
    Pants,
    TShirts,
    Layered,
    Faces,
    Animations,
    Body,
}

impl AvatarTab {
    pub const ALL: [Self; 11] = [
        Self::Wearing,
        Self::Outfits,
        Self::Accessories,
        Self::Hair,
        Self::Shirts,
        Self::Pants,
        Self::TShirts,
        Self::Layered,
        Self::Faces,
        Self::Animations,
        Self::Body,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Wearing => "Wearing",
            Self::Outfits => "Outfits",
            Self::Accessories => "Accessories",
            Self::Hair => "Hair",
            Self::Shirts => "Shirts",
            Self::Pants => "Pants",
            Self::TShirts => "T-Shirts",
            Self::Layered => "Layered",
            Self::Faces => "Faces & Heads",
            Self::Animations => "Animations",
            Self::Body => "Body & Skin",
        }
    }

    pub fn inventory_query(self) -> Option<&'static str> {
        match self {
            Self::Accessories => {
                Some("Hat,FaceAccessory,NeckAccessory,ShoulderAccessory,FrontAccessory,BackAccessory,WaistAccessory")
            }
            Self::Hair => Some("HairAccessory"),
            Self::Shirts => Some("Shirt"),
            Self::Pants => Some("Pants"),
            Self::TShirts => Some("TShirt"),
            Self::Layered => Some(
                "ShirtAccessory,TShirtAccessory,PantsAccessory,JacketAccessory,SweaterAccessory,ShortsAccessory,LeftShoeAccessory,RightShoeAccessory,DressSkirtAccessory",
            ),
            Self::Faces => Some("Face,DynamicHead"),
            Self::Animations => Some(
                "IdleAnimation,WalkAnimation,RunAnimation,JumpAnimation,FallAnimation,ClimbAnimation,SwimAnimation",
            ),
            Self::Wearing | Self::Outfits | Self::Body => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyColorTarget {
    All,
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

impl BodyColorTarget {
    pub const ALL: [Self; 7] = [
        Self::All,
        Self::Head,
        Self::Torso,
        Self::LeftArm,
        Self::RightArm,
        Self::LeftLeg,
        Self::RightLeg,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Head => "Head",
            Self::Torso => "Torso",
            Self::LeftArm => "L Arm",
            Self::RightArm => "R Arm",
            Self::LeftLeg => "L Leg",
            Self::RightLeg => "R Leg",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct OmniHome {
    pub continue_unis: Option<Vec<u64>>,
    pub topics: Vec<(String, Vec<u64>)>,
}

#[derive(Clone, Debug, Default)]
pub struct SavedSyncData {
    pub remote_recent: Option<Vec<Game>>,
    pub fallback_recent: Option<Vec<Game>>,
    pub remote_favs: Option<Vec<Game>>,
    pub fallback_favs: Option<Vec<Game>>,
}

#[derive(Clone, Debug, Default)]
pub struct UserSearchPage {
    pub users: Vec<User>,
    pub verified: HashSet<u64>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct FriendProfile {
    pub user: User,
    pub description: String,
    pub created: String,
    pub verified: bool,
    pub friends: u64,
    pub followers: u64,
    pub following: u64,
    pub avatar: String,
    pub playing_game: Option<Game>,
    pub experiences: Vec<Game>,
    pub currently_wearing: Vec<AvatarAsset>,
}

#[derive(Clone, Copy)]
pub struct CardPreview<'a> {
    pub universe_id: u64,
    pub texture: Option<&'a egui::TextureHandle>,
    pub is_loading: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NavIcon {
    Home,
    Charts,
    Profile,
    Avatar,
    Settings,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Page {
    Home,
    Charts,
    Search,
    Game(Game),
    Friend(User),
    AddFriend,
    Profile,
    Avatar,
    Settings,
    Login,
}

pub struct QuickLoginState {
    pub code: String,
    pub qr_uri: String,
    pub qr_bytes: Option<Vec<u8>>,
    pub status_text: String,
    pub error: Option<String>,
    pub cancel: Arc<AtomicBool>,
}

pub enum Action {
    Go(Page),
    GoChart(usize),
    RefreshGames,
    HoverGame(u64),
    Open(Game),
    OpenFriend(User),
    OpenAddFriend,
    SearchUsers(String),
    LoadMoreUsers,
    SendFriendRequest(u64, String),
    Play(Game),
    JoinPlace(u64),
    ToggleFav(Game),
    SelectGameTab(GameTab),
    GameMediaStep(i32),
    GameMediaSelect(usize),
    JoinServer(u64, String),
    VoteGame(u64, Option<bool>),
    RefreshServers(u64, u64),
    LoadMoreServers(u64, u64),
    SelectAvatarTab(AvatarTab),
    RefreshAvatar,
    RefreshAvatarTab(AvatarTab),
    LoadMoreAvatarTab(AvatarTab),
    ToggleWearAsset(AvatarAsset),
    UnequipAccessories,
    WearOutfit(OutfitItem),
    SetAvatarType(String),
    SetBodyColorId(u64),
    ApplyAvatarScales,
    ResetAvatarScales,
    DismissAvatarErr,
    RefreshMyProfile,
    SaveBio(String),
    OpenUrl(String),
    StartQuickLogin,
    SignOut,
}

pub enum Msg {
    Sorts(Result<Vec<Sort>, String>),
    Search(String, Result<Vec<Game>, String>),
    Lookup(Result<Game, String>),
    Details(Option<Game>),
    SavedSynced(SavedSyncData),
    RecentPlaceResolved(Game),
    Thumbs {
        wide: HashMap<u64, String>,
        icon: HashMap<u64, String>,
    },
    MyProfile(Result<FriendProfile, String>),
    BioSaved(Result<String, String>),
    FriendProfile(u64, Result<FriendProfile, String>),
    FriendSearchResult(bool, Result<UserSearchPage, String>),
    FriendRequestDone(u64, String, Result<(), String>),
    GameExtraLoaded(Box<GameExtra>),
    GamePassesLoaded(u64, Result<Vec<GamePassItem>, String>),
    GameServersLoaded(u64, bool, Result<(Vec<ServerItem>, Option<String>), String>),
    GameVoteDone(u64, Option<bool>, Result<(), String>),
    GameFavDone(u64, bool, Result<(), String>),
    AvatarLoaded(Result<(AvatarState, Vec<PaletteColor>), String>),
    AvatarInventory(AvatarTab, bool, Result<(Vec<AvatarAsset>, Option<String>), String>),
    AvatarOutfits(u32, Result<(Vec<OutfitItem>, bool), String>),
    AvatarApplied(u64, Result<(AvatarState, String, String, String), String>),
    Avatar3DLoaded(u64, Result<(avatar_3d::Avatar3DModel, Vec<(String, Vec<u8>)>), String>),
    AuthSession(Result<(Profile, Vec<User>), String>),
    QuickLoginCreated {
        code: String,
        qr_bytes: Option<Vec<u8>>,
    },
    QuickLoginStatus {
        code: String,
        status: String,
        account: Option<String>,
    },
    QuickLoginDone(Result<String, String>),
    ServerLogoutDone(Result<(), String>),
    LaunchResult(Result<String, String>),
    RobloxAppClosed,
    GameVideoIdResolved(u64, Option<u64>),
    VideoPreviewLoaded(u64, Result<Arc<video_preview::DecodedPreviewVideo>, String>),
}
