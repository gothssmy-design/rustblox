use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use eframe::egui::{self, RichText, Sense};

use crate::avatar_3d::{self, Avatar3DState, load_avatar_3d_data};
use crate::types::*;
use crate::theme::*;
use crate::auth::*;
use crate::roblox_process::*;
use crate::api::*;
use crate::video_preview;
use crate::ui::widgets::*;
use crate::ui::menu::*;

pub struct RustBlox {
    pub ctx: egui::Context,
    pub tx: Sender<Msg>,
    pub rx: Receiver<Msg>,
    pub sid: String,

    pub page: Page,
    pub back: Page,
    pub status: String,
    pub settings: Settings,
    pub player_path: Option<PathBuf>,
    pub handler: String,

    pub cookie: Option<String>,
    pub quick_login: Option<QuickLoginState>,

    pub search: String,
    pub searched: String,
    pub searching: bool,
    pub results: Option<Result<Vec<Game>, String>>,

    pub sorts: Option<Result<Vec<Sort>, String>>,
    pub chart: usize,

    pub recent_ids: Vec<u64>, // place ids
    pub recent: Vec<Game>,
    pub fav_ids: Vec<u64>, // place ids
    pub favs: Vec<Game>,

    pub wide: HashMap<u64, String>,
    pub icon: HashMap<u64, String>,
    pub requested: HashMap<u64, Instant>,
    pub game_video_ids: HashMap<u64, Option<u64>>,
    pub video_id_requested: HashSet<u64>,
    pub decoded_videos: HashMap<u64, Arc<video_preview::DecodedPreviewVideo>>,
    pub video_lru: Vec<u64>,
    pub video_decode_requested: HashSet<u64>,
    pub hovered_game_id: Option<u64>,
    pub hover_start: Option<Instant>,
    pub hover_play_start: Option<Instant>,
    pub active_video_texture: Option<(u64, u64, egui::TextureHandle)>,

    pub sorts_sync_started: Option<Instant>,
    pub saved_sync_started: Option<Instant>,
    pub auth_sync_started: Option<Instant>,
    pub last_sorts_sync: Option<Instant>,
    pub last_saved_sync: Option<Instant>,
    pub last_auth_sync: Option<Instant>,
    pub just_launched: Vec<(u64, Instant)>,
    pub pending_favs: HashMap<u64, (bool, Instant)>,
    pub was_focused: bool,
    pub auto_close_flag: Arc<AtomicBool>,
    pub watcher_stop: Arc<AtomicBool>,
    pub last_roblox_launch_ms: Arc<AtomicU64>,

    pub friend_names: Vec<String>,
    pub friends: Vec<User>,
    pub friends_err: Option<String>,
    pub adding_friend: bool,
    pub new_friend: String,
    pub friend_searched: String,
    pub friend_searching: bool,
    pub friend_search_loading_more: bool,
    pub friend_search_cursor: Option<String>,
    pub friend_search_verified: HashSet<u64>,
    pub friend_search_results: Option<Result<Vec<User>, String>>,
    pub friend_sent: HashSet<u64>,

    pub username: String,
    pub me: Option<Profile>,
    pub me_err: Option<String>,

    pub my_profile: Option<FriendProfile>,
    pub my_profile_loading: bool,
    pub my_profile_err: Option<String>,
    pub profile_3d_mode: bool,
    pub profile_bio_expanded: bool,
    pub profile_editing: bool,
    pub profile_edit_bio: String,
    pub profile_saving_bio: bool,
    pub profile_save_err: Option<String>,

    pub friend_profile: Option<FriendProfile>,
    pub friend_loading: bool,
    pub friend_err: Option<String>,

    pub avatar_state: Option<AvatarState>,
    pub avatar_palette: Vec<PaletteColor>,
    pub avatar_loading: bool,
    pub avatar_pending: u32,
    pub avatar_seq: u64,
    pub avatar_err: Option<String>,
    pub avatar_tab: AvatarTab,
    pub avatar_search: String,
    pub avatar_color_target: BodyColorTarget,
    pub avatar_items: HashMap<AvatarTab, Vec<AvatarAsset>>,
    pub avatar_cursors: HashMap<AvatarTab, Option<String>>,
    pub avatar_items_loading: Option<AvatarTab>,
    pub avatar_items_err: Option<String>,
    pub avatar_outfits: Option<Vec<OutfitItem>>,
    pub avatar_outfits_page: u32,
    pub avatar_outfits_more: bool,
    pub avatar_outfits_loading: bool,
    pub avatar_outfits_err: Option<String>,
    pub avatar_3d: Avatar3DState,

    pub game_tab: GameTab,
    pub game_extra: Option<GameExtra>,
    pub game_media_idx: usize,
    pub game_passes_loading: bool,
    pub game_passes_err: Option<String>,
    pub game_servers_loading: bool,
    pub game_servers_err: Option<String>,
    pub server_sort: ServerSort,
    pub server_exclude_full: bool,
}


impl RustBlox {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        let sid = new_session_id();
        let cookie = load_cookie();
        let logged_in = cookie.is_some();
        let settings = load_settings();
        install_fonts(&cc.egui_ctx);
        apply_theme(&cc.egui_ctx, settings.dark);
        let auto_close_flag = Arc::new(AtomicBool::new(settings.auto_close_roblox));
        let watcher_stop = Arc::new(AtomicBool::new(false));
        let last_roblox_launch_ms = Arc::new(AtomicU64::new(0));
        start_roblox_watcher(
            tx.clone(),
            cc.egui_ctx.clone(),
            Arc::clone(&auto_close_flag),
            Arc::clone(&watcher_stop),
            Arc::clone(&last_roblox_launch_ms),
        );
        let mut app = Self {
            ctx: cc.egui_ctx.clone(),
            tx,
            rx,
            sid,
            page: if logged_in { Page::Home } else { Page::Login },
            back: Page::Login,
            status: if logged_in {
                String::new()
            } else {
                "Sign in with Quick Login to use rustblox.".to_string()
            },
            settings,
            player_path: find_player(),
            handler: handler_status(),
            cookie,
            quick_login: None,
            search: String::new(),
            searched: String::new(),
            searching: false,
            results: None,
            sorts: None,
            chart: 0,
            recent_ids: read_ids("recent.txt"),
            recent: Vec::new(),
            fav_ids: read_ids("favorites.txt"),
            favs: Vec::new(),
            wide: HashMap::new(),
            icon: HashMap::new(),
            requested: HashMap::new(),
            game_video_ids: HashMap::new(),
            video_id_requested: HashSet::new(),
            decoded_videos: HashMap::new(),
            video_lru: Vec::new(),
            video_decode_requested: HashSet::new(),
            hovered_game_id: None,
            hover_start: None,
            hover_play_start: None,
            active_video_texture: None,
            sorts_sync_started: None,
            saved_sync_started: None,
            auth_sync_started: None,
            last_sorts_sync: None,
            last_saved_sync: None,
            last_auth_sync: None,
            just_launched: Vec::new(),
            pending_favs: HashMap::new(),
            was_focused: true,
            auto_close_flag,
            watcher_stop,
            last_roblox_launch_ms,
            friend_names: Vec::new(),
            friends: Vec::new(),
            friends_err: None,
            adding_friend: false,
            new_friend: String::new(),
            friend_searched: String::new(),
            friend_searching: false,
            friend_search_loading_more: false,
            friend_search_cursor: None,
            friend_search_verified: HashSet::new(),
            friend_search_results: None,
            friend_sent: HashSet::new(),
            username: String::new(),
            me: None,
            me_err: None,
            my_profile: None,
            my_profile_loading: false,
            my_profile_err: None,
            profile_3d_mode: false,
            profile_bio_expanded: false,
            profile_editing: false,
            profile_edit_bio: String::new(),
            profile_saving_bio: false,
            profile_save_err: None,
            friend_profile: None,
            friend_loading: false,
            friend_err: None,
            avatar_state: None,
            avatar_palette: Vec::new(),
            avatar_loading: false,
            avatar_pending: 0,
            avatar_seq: 0,
            avatar_err: None,
            avatar_tab: AvatarTab::Wearing,
            avatar_search: String::new(),
            avatar_color_target: BodyColorTarget::All,
            avatar_items: HashMap::new(),
            avatar_cursors: HashMap::new(),
            avatar_items_loading: None,
            avatar_items_err: None,
            avatar_outfits: None,
            avatar_outfits_page: 1,
            avatar_outfits_more: false,
            avatar_outfits_loading: false,
            avatar_outfits_err: None,
            avatar_3d: Avatar3DState::default(),
            game_tab: GameTab::About,
            game_extra: None,
            game_media_idx: 0,
            game_passes_loading: false,
            game_passes_err: None,
            game_servers_loading: false,
            game_servers_err: None,
            server_sort: ServerSort::Default,
            server_exclude_full: false,
        };
        if app.cookie.is_some() {
            app.sync_sorts(true);
            app.sync_saved(true);
            app.sync_auth_session(true);
        }
        app
    }

    pub fn spawn(&self, job: impl FnOnce() -> Msg + Send + 'static) {
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(job());
            ctx.request_repaint();
        });
    }

    pub fn is_sync_active(started: &Option<Instant>) -> bool {
        started.is_some_and(|t| t.elapsed() < Duration::from_secs(30))
    }

    pub fn is_stale(last: &Option<Instant>, started: &Option<Instant>, interval: Duration) -> bool {
        if Self::is_sync_active(started) {
            return false;
        }
        match (last, started) {
            (Some(t), _) => t.elapsed() >= interval,
            (None, None) => true,
            (None, Some(s)) => s.elapsed() >= interval,
        }
    }

    pub fn sync_sorts(&mut self, force: bool) {
        if self.cookie.is_none() {
            return;
        }
        if !force && Self::is_sync_active(&self.sorts_sync_started) {
            return;
        }
        self.sid = new_session_id();
        self.sorts_sync_started = Some(Instant::now());
        let sid = self.sid.clone();
        let cookie = self.cookie.clone();
        self.spawn(move || Msg::Sorts(fetch_sorts(&sid, cookie.as_deref())));
    }

    pub fn sync_saved(&mut self, force: bool) {
        let Some(cookie) = self.cookie.clone() else {
            return;
        };
        if !force && Self::is_sync_active(&self.saved_sync_started) {
            return;
        }
        self.saved_sync_started = Some(Instant::now());
        let uid = self.me.as_ref().map(|p| p.user.id);
        let r = self.recent_ids.clone();
        let f = self.fav_ids.clone();
        self.spawn(move || Msg::SavedSynced(fetch_saved_synced(&cookie, uid, r, f)));
    }

    pub fn sync_auth_session(&mut self, force: bool) {
        let Some(cookie) = self.cookie.clone() else {
            return;
        };
        if !force && Self::is_sync_active(&self.auth_sync_started) {
            return;
        }
        self.auth_sync_started = Some(Instant::now());
        self.spawn(move || Msg::AuthSession(fetch_authenticated_session(&cookie)));
    }

    pub fn force_refresh_games(&mut self) {
        if self.cookie.is_none() {
            return;
        }
        self.requested.clear();
        let thumb_ids: Vec<u64> = self
            .recent
            .iter()
            .chain(self.favs.iter())
            .map(|g| g.universe_id)
            .collect();
        self.request_thumbs(thumb_ids);
        self.sync_sorts(true);
        self.sync_saved(true);
        self.sync_auth_session(true);
    }

    pub fn maybe_auto_sync(&mut self, ctx: &egui::Context) {
        if self.cookie.is_none() {
            return;
        }
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
        let gained_focus = focused && !self.was_focused;
        self.was_focused = focused;

        ctx.request_repaint_after(Duration::from_secs(15));

        let on_discovery = matches!(self.page, Page::Home | Page::Charts);
        let saved_interval = if gained_focus {
            Duration::from_secs(5)
        } else if on_discovery {
            Duration::from_secs(45)
        } else {
            Duration::from_secs(90)
        };
        let sorts_interval = if gained_focus {
            Duration::from_secs(10)
        } else if on_discovery {
            Duration::from_secs(60)
        } else {
            Duration::from_secs(120)
        };
        let auth_interval = if gained_focus {
            Duration::from_secs(8)
        } else {
            Duration::from_secs(45)
        };

        if Self::is_stale(&self.last_saved_sync, &self.saved_sync_started, saved_interval) {
            self.sync_saved(false);
        }
        if Self::is_stale(&self.last_sorts_sync, &self.sorts_sync_started, sorts_interval) {
            self.sync_sorts(false);
        }
        if Self::is_stale(&self.last_auth_sync, &self.auth_sync_started, auth_interval) {
            self.sync_auth_session(false);
        }
    }

    pub fn find_known_game_by_place(&self, pid: u64) -> Option<Game> {
        if let Page::Game(g) = &self.page {
            if g.place_id == pid {
                return Some(g.clone());
            }
        }
        for g in self.recent.iter().chain(self.favs.iter()) {
            if g.place_id == pid {
                return Some(g.clone());
            }
        }
        if let Some(Ok(sorts)) = &self.sorts {
            for s in sorts {
                if let Some(g) = s.games.iter().find(|g| g.place_id == pid) {
                    return Some(g.clone());
                }
            }
        }
        if let Some(Ok(res)) = &self.results {
            if let Some(g) = res.iter().find(|g| g.place_id == pid) {
                return Some(g.clone());
            }
        }
        if let Some(g) = self
            .friend_profile
            .as_ref()
            .and_then(|fp| fp.playing_game.as_ref())
            .filter(|g| g.place_id == pid)
        {
            return Some(g.clone());
        }
        if let Some(g) = self
            .my_profile
            .as_ref()
            .and_then(|fp| fp.playing_game.as_ref())
            .filter(|g| g.place_id == pid)
        {
            return Some(g.clone());
        }
        None
    }

    pub fn record_recent_game(&mut self, g: Game) {
        let now = Instant::now();
        self.just_launched
            .retain(|(p, t)| *p != g.place_id && t.elapsed() < Duration::from_secs(600));
        self.just_launched.insert(0, (g.place_id, now));
        self.just_launched.truncate(8);

        self.recent_ids.retain(|p| *p != g.place_id);
        self.recent_ids.insert(0, g.place_id);
        self.recent_ids.truncate(24);
        write_ids("recent.txt", &self.recent_ids);

        self.recent.retain(|x| x.universe_id != g.universe_id && x.place_id != g.place_id);
        self.recent.insert(0, g.clone());
        self.recent.truncate(24);
        self.request_thumbs(vec![g.universe_id]);
    }

    pub fn record_recent_place_id(&mut self, pid: u64) {
        if pid == 0 {
            return;
        }
        if let Some(g) = self.find_known_game_by_place(pid) {
            self.record_recent_game(g);
            return;
        }
        let now = Instant::now();
        self.just_launched
            .retain(|(p, t)| *p != pid && t.elapsed() < Duration::from_secs(600));
        self.just_launched.insert(0, (pid, now));
        self.just_launched.truncate(8);

        self.recent_ids.retain(|p| *p != pid);
        self.recent_ids.insert(0, pid);
        self.recent_ids.truncate(24);
        write_ids("recent.txt", &self.recent_ids);

        self.spawn(move || {
            if let Some(g) = games_by_places(&[pid]).into_iter().next() {
                Msg::RecentPlaceResolved(g)
            } else {
                Msg::Details(None)
            }
        });
    }

    pub fn load_avatar(&mut self) {
        if self.avatar_loading {
            return;
        }
        if let Some(cookie) = self.cookie.clone() {
            self.avatar_loading = true;
            self.avatar_err = None;
            self.spawn(move || Msg::AvatarLoaded(fetch_avatar_session(&cookie)));
        }
    }

    pub fn load_avatar_3d(&mut self) {
        if self.avatar_3d.loading {
            return;
        }
        let Some(uid) = self.me.as_ref().map(|p| p.user.id) else {
            return;
        };
        let cookie = self.cookie.clone();
        self.avatar_3d.loading = true;
        self.avatar_3d.err = None;
        self.spawn(move || {
            let res = load_avatar_3d_data(uid, cookie.as_deref());
            Msg::Avatar3DLoaded(uid, res)
        });
    }

    pub fn ensure_avatar_loaded(&mut self) {
        if self.avatar_state.is_none() && !self.avatar_loading {
            self.load_avatar();
        }
        if self.avatar_3d.model.is_none() && !self.avatar_3d.loading {
            self.load_avatar_3d();
        }
        self.ensure_avatar_tab_loaded(self.avatar_tab);
    }

    pub fn ensure_my_profile_loaded(&mut self) {
        if self.my_profile_loading {
            return;
        }
        let Some(me) = self.me.as_ref() else {
            return;
        };
        let user = me.user.clone();
        let cookie = self.cookie.clone();
        self.my_profile_loading = true;
        self.my_profile_err = None;
        self.spawn(move || Msg::MyProfile(fetch_friend_profile(user, cookie)));
    }

    pub fn load_avatar_tab(&mut self, tab: AvatarTab, append: bool) {
        let Some(cookie) = self.cookie.clone() else {
            return;
        };
        let Some(uid) = self.me.as_ref().map(|p| p.user.id) else {
            return;
        };
        match tab {
            AvatarTab::Outfits => {
                if self.avatar_outfits_loading {
                    return;
                }
                let page = if append {
                    self.avatar_outfits_page + 1
                } else {
                    1
                };
                self.avatar_outfits_loading = true;
                self.avatar_outfits_err = None;
                self.spawn(move || Msg::AvatarOutfits(page, fetch_avatar_outfits(uid, page, &cookie)));
            }
            t if t.inventory_query().is_some() => {
                if self.avatar_items_loading == Some(t) {
                    return;
                }
                let cursor = if append {
                    let Some(c) = self.avatar_cursors.get(&t).and_then(|c| c.clone()) else {
                        return;
                    };
                    Some(c)
                } else {
                    None
                };
                self.avatar_items_loading = Some(t);
                self.avatar_items_err = None;
                self.spawn(move || {
                    Msg::AvatarInventory(
                        t,
                        append,
                        fetch_avatar_inventory(uid, t, &cookie, cursor.as_deref()),
                    )
                });
            }
            _ => {}
        }
    }

    pub fn ensure_avatar_tab_loaded(&mut self, tab: AvatarTab) {
        match tab {
            AvatarTab::Outfits if self.avatar_outfits.is_none() => self.load_avatar_tab(tab, false),
            t if t.inventory_query().is_some() && !self.avatar_items.contains_key(&t) => {
                self.load_avatar_tab(t, false)
            }
            _ => {}
        }
    }

    pub fn cancel_quick_login(&mut self) {
        if let Some(ql) = self.quick_login.take() {
            ql.cancel.store(true, Ordering::Relaxed);
            if !ql.qr_uri.is_empty() {
                self.ctx.forget_image(&ql.qr_uri);
            }
        }
    }

    pub fn begin_quick_login(&mut self) {
        self.cancel_quick_login();
        if !matches!(self.page, Page::Login) {
            self.back = self.page.clone();
        }
        self.page = Page::Login;
        let cancel = start_quick_login(self.tx.clone(), self.ctx.clone());
        self.quick_login = Some(QuickLoginState {
            code: String::new(),
            qr_uri: String::new(),
            qr_bytes: None,
            status_text: "Requesting Quick Login code...".to_string(),
            error: None,
            cancel,
        });
    }

    pub fn sign_out(&mut self) {
        self.cancel_quick_login();
        if let Some(old_cookie) = self.cookie.take() {
            self.spawn(move || {
                let res = post_csrf(
                    "https://auth.roblox.com/v2/logout",
                    serde_json::json!({}),
                    Some(&old_cookie),
                    Some("https://www.roblox.com/"),
                )
                .map(|_| ());
                Msg::ServerLogoutDone(res)
            });
        }
        clear_cookie();
        let _ = fs::remove_file(data_dir().join("username.txt"));
        self.username.clear();
        self.me = None;
        self.me_err = None;
        self.friends.clear();
        self.friends_err = None;
        self.friend_names.clear();
        self.adding_friend = false;
        self.new_friend.clear();
        self.friend_searched.clear();
        self.friend_searching = false;
        self.friend_search_loading_more = false;
        self.friend_search_cursor = None;
        self.friend_search_verified.clear();
        self.friend_search_results = None;
        self.friend_sent.clear();
        self.search.clear();
        self.searched.clear();
        self.searching = false;
        self.results = None;
        self.sorts = None;
        self.recent.clear();
        self.favs.clear();
        self.wide.clear();
        self.icon.clear();
        self.requested.clear();
        self.game_video_ids.clear();
        self.video_id_requested.clear();
        self.decoded_videos.clear();
        self.video_lru.clear();
        self.video_decode_requested.clear();
        self.hovered_game_id = None;
        self.hover_start = None;
        self.hover_play_start = None;
        self.active_video_texture = None;
        self.sorts_sync_started = None;
        self.saved_sync_started = None;
        self.auth_sync_started = None;
        self.last_sorts_sync = None;
        self.last_saved_sync = None;
        self.last_auth_sync = None;
        self.just_launched.clear();
        self.pending_favs.clear();
        self.my_profile = None;
        self.my_profile_loading = false;
        self.my_profile_err = None;
        self.profile_3d_mode = false;
        self.profile_bio_expanded = false;
        self.profile_editing = false;
        self.profile_edit_bio.clear();
        self.profile_saving_bio = false;
        self.profile_save_err = None;
        self.friend_profile = None;
        self.friend_loading = false;
        self.friend_err = None;
        self.avatar_state = None;
        self.avatar_palette.clear();
        self.avatar_loading = false;
        self.avatar_pending = 0;
        self.avatar_seq = 0;
        self.avatar_err = None;
        self.avatar_tab = AvatarTab::Wearing;
        self.avatar_search.clear();
        self.avatar_color_target = BodyColorTarget::All;
        self.avatar_items.clear();
        self.avatar_cursors.clear();
        self.avatar_items_loading = None;
        self.avatar_items_err = None;
        self.avatar_outfits = None;
        self.avatar_outfits_page = 1;
        self.avatar_outfits_more = false;
        self.avatar_outfits_loading = false;
        self.avatar_outfits_err = None;
        self.avatar_3d = Avatar3DState::default();
        self.game_tab = GameTab::About;
        self.game_extra = None;
        self.game_media_idx = 0;
        self.game_passes_loading = false;
        self.game_passes_err = None;
        self.game_servers_loading = false;
        self.game_servers_err = None;
        self.server_sort = ServerSort::Default;
        self.server_exclude_full = false;
        self.ctx.forget_all_images();
        self.page = Page::Login;
        self.back = Page::Login;
        self.status = "Signed out â€” sign in with Quick Login to continue.".to_string();
    }

    pub fn request_thumbs(&mut self, ids: Vec<u64>) {
        let now = Instant::now();
        let retry_missing = Duration::from_secs(25);
        let refresh_existing = Duration::from_secs(300);
        let mut new = Vec::new();
        let mut queued = HashSet::new();
        for id in ids {
            if id == 0 || !queued.insert(id) {
                continue;
            }
            let have_both = self.wide.contains_key(&id) && self.icon.contains_key(&id);
            let min_age = if have_both {
                refresh_existing
            } else {
                retry_missing
            };
            let ready = self.requested.get(&id).is_none_or(|t| t.elapsed() >= min_age);
            if ready {
                self.requested.insert(id, now);
                new.push(id);
            }
        }
        if !new.is_empty() {
            self.spawn(move || fetch_thumbs(new));
        }
    }

    pub fn start_search(&mut self) {
        let q = self.search.trim().to_string();
        if q.is_empty() {
            return;
        }
        self.searched = q.clone();
        self.searching = true;
        self.results = None;
        self.page = Page::Search;
        if let Ok(place) = q.parse::<u64>() {
            // A number is treated as a Place ID
            self.spawn(move || {
                Msg::Lookup(
                    games_by_places(&[place])
                        .into_iter()
                        .next()
                        .ok_or_else(|| "No game with that Place ID".to_string()),
                )
            });
        } else {
            let sid = new_session_id();
            self.sid = sid.clone();
            self.spawn(move || {
                let r = search_games(&q, &sid);
                Msg::Search(q, r)
            });
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut had_messages = false;
        while let Ok(msg) = self.rx.try_recv() {
            had_messages = true;
            let logged_in = self.cookie.is_some();
            match msg {
                Msg::Sorts(r) => {
                    if !logged_in {
                        continue;
                    }
                    self.sorts_sync_started = None;
                    self.last_sorts_sync = Some(Instant::now());
                    match r {
                        Ok(s) => {
                            let ids: Vec<u64> = s
                                .iter()
                                .flat_map(|sort| sort.games.iter().map(|g| g.universe_id))
                                .collect();
                            if !s.is_empty() {
                                if self.chart >= s.len() {
                                    self.chart = 0;
                                }
                                self.sorts = Some(Ok(s));
                            } else if self.sorts.is_none() || matches!(self.sorts, Some(Err(_))) {
                                self.sorts = Some(Ok(s));
                            }
                            self.request_thumbs(ids);
                        }
                        Err(e) => {
                            if self.sorts.is_none() || matches!(self.sorts, Some(Err(_))) {
                                self.sorts = Some(Err(e));
                            }
                        }
                    }
                }
                Msg::Search(q, r) => {
                    if !logged_in {
                        continue;
                    }
                    if q == self.searched {
                        self.searching = false;
                        if let Ok(v) = &r {
                            let ids = v.iter().map(|g| g.universe_id).collect();
                            self.request_thumbs(ids);
                        }
                        self.results = Some(r);
                    }
                }
                Msg::Lookup(r) => {
                    if !logged_in {
                        continue;
                    }
                    self.searching = false;
                    match r {
                        Ok(g) => {
                            self.back = Page::Home;
                            self.open_game_details(g);
                        }
                        Err(e) => self.results = Some(Err(e)),
                    }
                }
                Msg::Details(Some(full)) => {
                    if !logged_in {
                        continue;
                    }
                    if let Page::Game(cur) = &mut self.page {
                        if cur.universe_id == full.universe_id {
                            *cur = full;
                        }
                    }
                }
                Msg::Details(None) => {}
                Msg::GameExtraLoaded(ex) => {
                    if !logged_in {
                        continue;
                    }
                    self.game_video_ids.insert(ex.universe_id, ex.preview_video_id);
                    if let Page::Game(cur) = &self.page {
                        if cur.universe_id == ex.universe_id {
                            self.game_extra = Some(*ex);
                        }
                    }
                }
                Msg::GameVideoIdResolved(uid, vid_opt) => {
                    self.video_id_requested.remove(&uid);
                    self.game_video_ids.insert(uid, vid_opt);
                    if let Some(vid) = vid_opt {
                        if !self.decoded_videos.contains_key(&vid)
                            && !self.video_decode_requested.contains(&vid)
                        {
                            self.video_decode_requested.insert(vid);
                            let cookie = self.cookie.clone();
                            self.spawn(move || {
                                let res = fetch_and_decode_preview_video(vid, 360, cookie.as_deref()).map(Arc::new);
                                Msg::VideoPreviewLoaded(vid, res)
                            });
                        }
                    }
                }
                Msg::VideoPreviewLoaded(vid, res) => {
                    self.video_decode_requested.remove(&vid);
                    if let Ok(video) = res {
                        if !self.decoded_videos.contains_key(&vid) {
                            if self.video_lru.len() >= 8 {
                                if let Some(old_vid) = self.video_lru.pop() {
                                    self.decoded_videos.remove(&old_vid);
                                }
                            }
                            self.video_lru.insert(0, vid);
                            self.decoded_videos.insert(vid, video);
                        }
                        if self.hover_play_start.is_none() {
                            self.hover_play_start = Some(Instant::now());
                        }
                    }
                }
                Msg::GamePassesLoaded(uid, r) => {
                    if !logged_in {
                        continue;
                    }
                    if matches!(&self.page, Page::Game(cur) if cur.universe_id == uid) {
                        self.game_passes_loading = false;
                        match r {
                            Ok(passes) => {
                                if let Some(ex) = &mut self.game_extra {
                                    ex.passes = passes;
                                    ex.passes_loaded = true;
                                }
                                self.game_passes_err = None;
                            }
                            Err(e) => {
                                self.game_passes_err = Some(e);
                            }
                        }
                    }
                }
                Msg::GameServersLoaded(uid, append, r) => {
                    if !logged_in {
                        continue;
                    }
                    if matches!(&self.page, Page::Game(cur) if cur.universe_id == uid) {
                        self.game_servers_loading = false;
                        match r {
                            Ok((mut list, next_cursor)) => {
                                if let Some(ex) = &mut self.game_extra {
                                    if append {
                                        ex.servers.append(&mut list);
                                    } else {
                                        ex.servers = list;
                                    }
                                    ex.servers_cursor = next_cursor;
                                    ex.servers_loaded = true;
                                }
                                self.game_servers_err = None;
                            }
                            Err(e) => {
                                self.game_servers_err = Some(e);
                            }
                        }
                    }
                }
                Msg::GameVoteDone(uid, vote, r) => {
                    if !logged_in {
                        continue;
                    }
                    match r {
                        Ok(()) => {
                            self.status = match vote {
                                Some(true) => "Upvoted experience.".to_string(),
                                Some(false) => "Downvoted experience.".to_string(),
                                None => "Cleared vote.".to_string(),
                            };
                        }
                        Err(e) => {
                            self.status = format!("Could not submit vote: {e}");
                            let cookie = self.cookie.clone();
                            self.spawn(move || {
                                Msg::GameExtraLoaded(Box::new(fetch_game_extra(
                                    uid,
                                    cookie.as_deref(),
                                )))
                            });
                        }
                    }
                }
                Msg::GameFavDone(uid, is_fav, r) => {
                    if !logged_in {
                        continue;
                    }
                    match r {
                        Ok(()) => {
                            if let Some(ex) = &mut self.game_extra {
                                if ex.universe_id == uid {
                                    ex.is_favorited_api = Some(is_fav);
                                }
                            }
                        }
                        Err(e) => {
                            self.status = format!("Saved locally (Roblox favorite sync: {e})");
                        }
                    }
                }
                Msg::SavedSynced(sync) => {
                    if !logged_in {
                        continue;
                    }
                    self.saved_sync_started = None;
                    self.last_saved_sync = Some(Instant::now());
                    let mut thumb_ids = Vec::new();

                    self.just_launched
                        .retain(|(_, t)| t.elapsed() < Duration::from_secs(600));
                    let next_recent = if let Some(remote) = sync.remote_recent {
                        if !remote.is_empty() || self.just_launched.is_empty() {
                            let mut merged = Vec::new();
                            let mut fallback_by_pid: HashMap<u64, Game> = self
                                .recent
                                .iter()
                                .chain(sync.fallback_recent.iter().flatten())
                                .map(|g| (g.place_id, g.clone()))
                                .collect();
                            for (pid, _) in &self.just_launched {
                                if !remote.iter().any(|g| g.place_id == *pid)
                                    && !merged.iter().any(|g: &Game| g.place_id == *pid)
                                {
                                    if let Some(g) = fallback_by_pid.remove(pid) {
                                        merged.push(g);
                                    }
                                }
                            }
                            for g in remote {
                                if !merged.iter().any(|x| x.universe_id == g.universe_id) {
                                    merged.push(g);
                                }
                            }
                            merged.truncate(24);
                            Some(merged)
                        } else {
                            sync.fallback_recent
                        }
                    } else {
                        sync.fallback_recent
                    };
                    if let Some(v) = next_recent {
                        thumb_ids.extend(v.iter().map(|g| g.universe_id));
                        let next_ids: Vec<u64> =
                            v.iter().filter(|g| g.place_id > 0).map(|g| g.place_id).collect();
                        if !next_ids.is_empty() || !self.recent.is_empty() {
                            self.recent_ids = next_ids;
                            write_ids("recent.txt", &self.recent_ids);
                        }
                        self.recent = v;
                    }

                    self.pending_favs
                        .retain(|_, (_, t)| t.elapsed() < Duration::from_secs(15));
                    let next_favs = if let Some(mut remote) = sync.remote_favs {
                        for (uid, (want_fav, _)) in &self.pending_favs {
                            if *want_fav {
                                if !remote.iter().any(|g| g.universe_id == *uid) {
                                    if let Some(existing) =
                                        self.favs.iter().find(|g| g.universe_id == *uid)
                                    {
                                        remote.insert(0, existing.clone());
                                    }
                                }
                            } else {
                                remote.retain(|g| g.universe_id != *uid);
                            }
                        }
                        remote.truncate(24);
                        Some(remote)
                    } else {
                        sync.fallback_favs
                    };
                    if let Some(v) = next_favs {
                        thumb_ids.extend(v.iter().map(|g| g.universe_id));
                        let next_ids: Vec<u64> =
                            v.iter().filter(|g| g.place_id > 0).map(|g| g.place_id).collect();
                        self.fav_ids = next_ids;
                        write_ids("favorites.txt", &self.fav_ids);
                        self.favs = v;
                    }

                    self.request_thumbs(thumb_ids);
                }
                Msg::RecentPlaceResolved(g) => {
                    if !logged_in {
                        continue;
                    }
                    self.recent
                        .retain(|x| x.universe_id != g.universe_id && x.place_id != g.place_id);
                    self.recent.insert(0, g.clone());
                    self.recent.truncate(24);
                    self.request_thumbs(vec![g.universe_id]);
                }
                Msg::Thumbs { wide, icon } => {
                    if !logged_in {
                        continue;
                    }
                    for (uid, url) in wide {
                        if let Some(old) = self.wide.get(&uid) {
                            if old != &url {
                                self.ctx.forget_image(old);
                            }
                        }
                        self.wide.insert(uid, url);
                    }
                    for (uid, url) in icon {
                        if let Some(old) = self.icon.get(&uid) {
                            if old != &url {
                                self.ctx.forget_image(old);
                            }
                        }
                        self.icon.insert(uid, url);
                    }
                }
                Msg::MyProfile(r) => {
                    self.my_profile_loading = false;
                    match r {
                        Ok(fp) => {
                            let mut ids: Vec<u64> =
                                fp.experiences.iter().map(|g| g.universe_id).collect();
                            if let Some(g) = &fp.playing_game {
                                ids.push(g.universe_id);
                            }
                            self.request_thumbs(ids);
                            self.my_profile = Some(fp);
                            self.my_profile_err = None;
                        }
                        Err(e) => {
                            self.my_profile = None;
                            self.my_profile_err = Some(e);
                        }
                    }
                }
                Msg::BioSaved(r) => {
                    self.profile_saving_bio = false;
                    match r {
                        Ok(new_bio) => {
                            if let Some(p) = &mut self.my_profile {
                                p.description = new_bio;
                            }
                            self.profile_editing = false;
                            self.profile_save_err = None;
                            self.status = "Bio updated successfully!".to_string();
                        }
                        Err(e) => {
                            self.profile_save_err = Some(e);
                        }
                    }
                }
                Msg::FriendProfile(uid, r) => {
                    if !logged_in {
                        continue;
                    }
                    if matches!(&self.page, Page::Friend(u) if u.id == uid) {
                        self.friend_loading = false;
                        match r {
                            Ok(fp) => {
                                let mut ids: Vec<u64> =
                                    fp.experiences.iter().map(|g| g.universe_id).collect();
                                if let Some(g) = &fp.playing_game {
                                    ids.push(g.universe_id);
                                }
                                self.request_thumbs(ids);
                                self.friend_profile = Some(fp);
                                self.friend_err = None;
                            }
                            Err(e) => {
                                self.friend_profile = None;
                                self.friend_err = Some(e);
                            }
                        }
                    }
                }
                Msg::FriendSearchResult(append, r) => {
                    if !logged_in {
                        continue;
                    }
                    self.friend_searching = false;
                    self.friend_search_loading_more = false;
                    match r {
                        Ok(page) => {
                            self.friend_search_cursor = page.next_cursor;
                            self.friend_search_verified.extend(page.verified);
                            if append {
                                if let Some(Ok(existing)) = &mut self.friend_search_results {
                                    let seen: HashSet<u64> = existing.iter().map(|u| u.id).collect();
                                    for u in page.users {
                                        if !seen.contains(&u.id) {
                                            existing.push(u);
                                        }
                                    }
                                }
                            } else {
                                self.friend_search_results = Some(Ok(page.users));
                            }
                        }
                        Err(e) => {
                            if !append {
                                self.friend_search_results = Some(Err(e));
                                self.friend_search_cursor = None;
                            } else {
                                self.status = format!("Could not load more players: {e}");
                            }
                        }
                    }
                }
                Msg::FriendRequestDone(uid, uname, r) => {
                    if !logged_in {
                        continue;
                    }
                    match r {
                        Ok(()) => {
                            self.friend_sent.insert(uid);
                            self.status = format!("Friend request sent to @{uname}!");
                        }
                        Err(e) => {
                            self.status = format!("Could not add @{uname}: {e}");
                        }
                    }
                }
                Msg::AvatarLoaded(r) => {
                    if !logged_in {
                        continue;
                    }
                    self.avatar_loading = false;
                    match r {
                        Ok((state, palette)) => {
                            self.avatar_state = Some(state);
                            if !palette.is_empty() {
                                self.avatar_palette = palette;
                            }
                            self.avatar_err = None;
                        }
                        Err(e) => {
                            self.avatar_err = Some(e);
                        }
                    }
                }
                Msg::AvatarInventory(tab, append, r) => {
                    if !logged_in {
                        continue;
                    }
                    if self.avatar_items_loading == Some(tab) {
                        self.avatar_items_loading = None;
                    }
                    match r {
                        Ok((mut items, next_cursor)) => {
                            if append {
                                let entry = self.avatar_items.entry(tab).or_default();
                                entry.append(&mut items);
                            } else {
                                self.avatar_items.insert(tab, items);
                            }
                            self.avatar_cursors.insert(tab, next_cursor);
                            if self.avatar_tab == tab {
                                self.avatar_items_err = None;
                            }
                        }
                        Err(e) => {
                            if self.avatar_tab == tab {
                                self.avatar_items_err = Some(e);
                            }
                        }
                    }
                }
                Msg::AvatarOutfits(page, r) => {
                    if !logged_in {
                        continue;
                    }
                    self.avatar_outfits_loading = false;
                    match r {
                        Ok((mut outfits, has_more)) => {
                            self.avatar_outfits_page = page;
                            self.avatar_outfits_more = has_more;
                            if page > 1 {
                                let list = self.avatar_outfits.get_or_insert_with(Vec::new);
                                list.append(&mut outfits);
                            } else {
                                self.avatar_outfits = Some(outfits);
                            }
                            self.avatar_outfits_err = None;
                        }
                        Err(e) => {
                            self.avatar_outfits_err = Some(e);
                        }
                    }
                }
                Msg::AvatarApplied(seq, r) => {
                    if !logged_in {
                        continue;
                    }
                    self.avatar_pending = self.avatar_pending.saturating_sub(1);
                    match r {
                        Ok((state, avatar_url, headshot_url, note)) => {
                            if seq == self.avatar_seq {
                                self.avatar_state = Some(state);
                                self.avatar_err = None;
                            }
                            if let Some(prof) = &mut self.me {
                                if !avatar_url.is_empty() {
                                    if !prof.avatar.is_empty() && prof.avatar != avatar_url {
                                        self.ctx.forget_image(&prof.avatar);
                                    }
                                    prof.avatar = avatar_url;
                                }
                                if !headshot_url.is_empty() {
                                    if !prof.user.headshot.is_empty()
                                        && prof.user.headshot != headshot_url
                                    {
                                        self.ctx.forget_image(&prof.user.headshot);
                                    }
                                    prof.user.headshot = headshot_url;
                                }
                            }
                            if seq == self.avatar_seq {
                                self.status = note;
                                self.avatar_3d.loaded_user_id = None;
                                self.load_avatar_3d();
                            }
                        }
                        Err(e) => {
                            if seq == self.avatar_seq {
                                self.avatar_err = Some(e.clone());
                                self.status = format!("Avatar update failed: {e}");
                                self.load_avatar();
                            }
                        }
                    }
                }
                Msg::Avatar3DLoaded(uid, res) => {
                    self.avatar_3d.loading = false;
                    match res {
                        Ok((model, raw_textures)) => {
                            self.avatar_3d.loaded_user_id = Some(uid);
                            self.avatar_3d.model = Some(model);
                            self.avatar_3d.err = None;
                            self.avatar_3d.textures.clear();
                            for (hash, bytes) in raw_textures {
                                if let Ok(img) = image::load_from_memory(&bytes) {
                                    let size = [img.width() as _, img.height() as _];
                                    let rgba = img.to_rgba8();
                                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                        size,
                                        rgba.as_flat_samples().as_slice(),
                                    );
                                    self.avatar_3d.textures.push(avatar_3d::LoadedTexture {
                                        hash,
                                        image: color_img,
                                        handle: None,
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            self.avatar_3d.err = Some(e);
                        }
                    }
                }
                Msg::AuthSession(r) => {
                    if !logged_in {
                        continue;
                    }
                    self.auth_sync_started = None;
                    self.last_auth_sync = Some(Instant::now());
                    match r {
                        Ok((profile, friends)) => {
                            self.status.clear();
                            if let Some(prev) = &self.me {
                                if prev.user.headshot != profile.user.headshot
                                    && !prev.user.headshot.is_empty()
                                {
                                    self.ctx.forget_image(&prev.user.headshot);
                                }
                            }
                            let prev_shots: HashMap<u64, String> = self
                                .friends
                                .iter()
                                .map(|u| (u.id, u.headshot.clone()))
                                .collect();
                            for f in &friends {
                                if let Some(old) = prev_shots.get(&f.id) {
                                    if old != &f.headshot && !old.is_empty() {
                                        self.ctx.forget_image(old);
                                    }
                                }
                            }
                            self.me = Some(profile);
                            self.me_err = None;
                            self.friends = friends;
                            self.friends_err = None;
                            if matches!(self.page, Page::Avatar) {
                                self.ensure_avatar_loaded();
                            }
                            if matches!(self.page, Page::Profile) {
                                self.ensure_my_profile_loaded();
                            }
                        }
                        Err(e) => {
                            if e.contains("Session expired") || e.contains("401") || e.contains("403")
                            {
                                self.sign_out();
                                self.status = format!("Session expired â€” please sign in again ({e})");
                            } else if self.me.is_none() {
                                self.me_err = Some(e.clone());
                                self.status = e;
                            }
                        }
                    }
                }
                Msg::QuickLoginCreated { code, qr_bytes } => {
                    if let Some(ql) = &mut self.quick_login {
                        ql.qr_uri = format!("bytes://ql_qr_{code}.png");
                        ql.code = code;
                        ql.qr_bytes = qr_bytes;
                        ql.status_text =
                            "Waiting for you to enter this code in Roblox...".to_string();
                    }
                }
                Msg::QuickLoginStatus {
                    code,
                    status,
                    account,
                } => {
                    if let Some(ql) = &mut self.quick_login {
                        if ql.code == code {
                            ql.status_text = match (status.as_str(), account) {
                                ("UserLinked", Some(acc)) => {
                                    format!("Linked to {acc}! Confirm on your device to finish...")
                                }
                                ("UserLinked", None) => {
                                    "Code linked! Confirm on your device to finish...".to_string()
                                }
                                _ => "Waiting for you to enter this code in Roblox...".to_string(),
                            };
                        }
                    }
                }
                Msg::QuickLoginDone(r) => match r {
                    Ok(cookie) => {
                        self.cancel_quick_login();
                        if let Err(e) = save_cookie(&cookie) {
                            self.status = format!("Signed in, but could not save session: {e}");
                        } else {
                            self.status = "Signed in! Loading your Roblox profile...".to_string();
                        }
                        self.cookie = Some(cookie);
                        self.page = Page::Home;
                        self.back = Page::Home;
                        self.sync_sorts(true);
                        self.sync_saved(true);
                        self.sync_auth_session(true);
                    }
                    Err(e) => {
                        if let Some(ql) = &mut self.quick_login {
                            ql.error = Some(e);
                            ql.status_text.clear();
                        }
                    }
                },
                Msg::ServerLogoutDone(r) => {
                    if self.cookie.is_none() {
                        self.status = match r {
                            Ok(()) => "Signed out â€” session revoked on Roblox.".to_string(),
                            Err(_) => "Signed out locally (could not reach Roblox to revoke token).".to_string(),
                        };
                    }
                }
                Msg::LaunchResult(r) => {
                    self.status = match r {
                        Ok(s) => s,
                        Err(e) => format!("Launch error: {e}"),
                    };
                }
                Msg::RobloxAppClosed => {
                    if self.status.contains("Launching")
                        || self.status.contains("Joining")
                        || self.status.contains("Getting launch ticket")
                    {
                        self.status = "Closed Roblox app after session.".to_string();
                    }
                    if logged_in {
                        self.sync_saved(true);
                        self.sync_auth_session(true);
                    }
                }
            }
        }
        had_messages
    }

    pub fn current_card_preview(&self) -> Option<CardPreview<'_>> {
        let uid = self.hovered_game_id?;
        let vid_opt = self.game_video_ids.get(&uid);
        let is_loading = match vid_opt {
            Some(Some(vid)) => !self.decoded_videos.contains_key(vid),
            None => self.video_id_requested.contains(&uid),
            Some(None) => false,
        };
        let tex = if let Some((active_uid, _, tex)) = &self.active_video_texture {
            if *active_uid == uid {
                Some(tex)
            } else {
                None
            }
        } else {
            None
        };
        Some(CardPreview {
            universe_id: uid,
            texture: tex,
            is_loading,
        })
    }

    pub fn handle_game_hover(&mut self, hovered: Option<u64>, ctx: &egui::Context) {
        let now = Instant::now();
        if hovered != self.hovered_game_id {
            self.hovered_game_id = hovered;
            self.hover_start = hovered.map(|_| now);
            self.hover_play_start = None;
            self.active_video_texture = None;
        }

        let Some(uid) = self.hovered_game_id else {
            return;
        };

        // If video is already decoded in memory, start playing immediately!
        let has_cached_video = self
            .game_video_ids
            .get(&uid)
            .and_then(|opt| *opt)
            .map(|vid| self.decoded_videos.contains_key(&vid))
            .unwrap_or(false);

        if !has_cached_video {
            let dwell_passed = self
                .hover_start
                .map(|t| now.duration_since(t).as_millis() >= 75)
                .unwrap_or(false);
            if !dwell_passed {
                ctx.request_repaint_after(Duration::from_millis(40));
                return;
            }
        }

        // 1. Resolve video_id if not yet known
        match self.game_video_ids.get(&uid) {
            None => {
                if !self.video_id_requested.contains(&uid) {
                    self.video_id_requested.insert(uid);
                    self.spawn(move || {
                        let vid = fetch_game_preview_video_id(uid);
                        Msg::GameVideoIdResolved(uid, vid)
                    });
                }
                ctx.request_repaint_after(Duration::from_millis(60));
            }
            Some(None) => {
                // Confirmed no video for this game
            }
            Some(Some(vid)) => {
                let vid = *vid;
                // 2. Fetch and decode if not in decoded_videos
                if let Some(video) = self.decoded_videos.get(&vid) {
                    if self.hover_play_start.is_none() {
                        self.hover_play_start = Some(now);
                    }
                    let elapsed_ms = self.hover_play_start.unwrap().elapsed().as_millis() as u64;
                    if let Some(frame) = video.frame_at(elapsed_ms) {
                        let mut need_new_tex = false;
                        if let Some((cur_uid, cur_ts, tex)) = &mut self.active_video_texture {
                            if *cur_uid == uid {
                                if *cur_ts != frame.timestamp_ms {
                                    tex.set(frame.image.clone(), egui::TextureOptions::LINEAR);
                                    *cur_ts = frame.timestamp_ms;
                                }
                            } else {
                                need_new_tex = true;
                            }
                        } else {
                            need_new_tex = true;
                        }

                        if need_new_tex {
                            let tex_name = format!("preview_video_{vid}");
                            let tex = ctx.load_texture(
                                tex_name,
                                frame.image.clone(),
                                egui::TextureOptions::LINEAR,
                            );
                            self.active_video_texture = Some((uid, frame.timestamp_ms, tex));
                        }

                        // Schedule next frame redraw
                        ctx.request_repaint_after(Duration::from_millis(33));
                    }
                } else if !self.video_decode_requested.contains(&vid) {
                    self.video_decode_requested.insert(vid);
                    let cookie = self.cookie.clone();
                    self.spawn(move || {
                        let res = fetch_and_decode_preview_video(vid, 360, cookie.as_deref()).map(Arc::new);
                        Msg::VideoPreviewLoaded(vid, res)
                    });
                    ctx.request_repaint_after(Duration::from_millis(60));
                } else {
                    ctx.request_repaint_after(Duration::from_millis(60));
                }
            }
        }
    }

    pub fn open_game_details(&mut self, g: Game) {
        if !matches!(self.page, Page::Game(_)) {
            self.back = self.page.clone();
        }
        let uid = g.universe_id;
        let cookie = self.cookie.clone();
        self.request_thumbs(vec![uid]);
        self.game_tab = GameTab::About;
        self.game_extra = None;
        self.game_media_idx = 0;
        self.game_passes_loading = false;
        self.game_passes_err = None;
        self.game_servers_loading = false;
        self.game_servers_err = None;
        self.server_sort = ServerSort::Default;
        self.server_exclude_full = false;
        self.page = Page::Game(g);
        self.spawn(move || Msg::Details(games_by_universe(&[uid]).into_iter().next()));
        self.spawn(move || Msg::GameExtraLoaded(Box::new(fetch_game_extra(uid, cookie.as_deref()))));
    }

    pub fn load_game_passes(&mut self, universe_id: u64) {
        if self.game_passes_loading {
            return;
        }
        let cookie = self.cookie.clone();
        self.game_passes_loading = true;
        self.game_passes_err = None;
        self.spawn(move || {
            Msg::GamePassesLoaded(universe_id, fetch_game_passes(universe_id, cookie.as_deref()))
        });
    }

    pub fn load_game_servers(&mut self, universe_id: u64, place_id: u64, append: bool) {
        if self.game_servers_loading {
            return;
        }
        let cursor = if append {
            let Some(c) = self
                .game_extra
                .as_ref()
                .filter(|ex| ex.universe_id == universe_id)
                .and_then(|ex| ex.servers_cursor.clone())
            else {
                return;
            };
            Some(c)
        } else {
            None
        };
        let cookie = self.cookie.clone();
        self.game_servers_loading = true;
        self.game_servers_err = None;
        self.spawn(move || {
            Msg::GameServersLoaded(
                universe_id,
                append,
                fetch_game_servers(place_id, cursor.as_deref(), cookie.as_deref()),
            )
        });
    }

    pub fn launch_place_id(&self, place_id: u64) {
        if let Some(cookie) = self.cookie.clone() {
            self.last_roblox_launch_ms
                .store(now_unix_ms(), Ordering::Relaxed);
            self.spawn(move || Msg::LaunchResult(launch_authenticated(place_id, &cookie)));
        }
    }

    pub fn launch_server(&self, place_id: u64, job_id: String) {
        if let Some(cookie) = self.cookie.clone() {
            self.last_roblox_launch_ms
                .store(now_unix_ms(), Ordering::Relaxed);
            self.spawn(move || {
                Msg::LaunchResult(launch_authenticated_server(place_id, Some(&job_id), &cookie))
            });
        }
    }

    pub fn play(&mut self, g: &Game) {
        if self.cookie.is_none() {
            return;
        }
        self.status = "Getting launch ticket...".to_string();
        self.launch_place_id(g.place_id);
        self.record_recent_game(g.clone());
    }

    // ----- pages -----

    pub fn toggle_fav(&mut self, g: &Game) {
        let now_fav = if self.fav_ids.contains(&g.place_id) {
            self.fav_ids.retain(|p| *p != g.place_id);
            self.favs.retain(|x| x.universe_id != g.universe_id);
            self.status = format!("Removed {} from favorites", g.name);
            false
        } else {
            self.fav_ids.push(g.place_id);
            self.favs.retain(|x| x.universe_id != g.universe_id);
            self.favs.insert(0, g.clone());
            self.status = format!("Added {} to favorites", g.name);
            true
        };
        self.pending_favs
            .insert(g.universe_id, (now_fav, Instant::now()));
        if let Some(ex) = &mut self.game_extra {
            if ex.universe_id == g.universe_id {
                ex.is_favorited_api = Some(now_fav);
                ex.favorites = if now_fav {
                    ex.favorites.saturating_add(1)
                } else {
                    ex.favorites.saturating_sub(1)
                };
            }
        }
        write_ids("favorites.txt", &self.fav_ids);
        if let Some(cookie) = self.cookie.clone() {
            let uid = g.universe_id;
            self.spawn(move || Msg::GameFavDone(uid, now_fav, favorite_game_api(uid, now_fav, &cookie)));
        }
    }

}

impl eframe::App for RustBlox {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.poll() {
            ui.ctx().request_repaint();
        }
        self.maybe_auto_sync(ui.ctx());
        let logged_in = self.cookie.is_some();
        let p = pal(ui);
        let mut acts: Vec<Action> = Vec::new();
        let mut do_search = false;
        let mut toggle_theme = false;

        // Left sidebar (only shown when logged in)
        if logged_in {
            egui::Panel::left("nav")
                .resizable(false)
                .exact_size(74.0)
                .show_separator_line(false)
                .frame(egui::Frame::new().fill(p.bg).inner_margin(4))
                .show(ui, |ui| {
                    ui.add_space(10.0);
                    let page = &self.page;
                    if nav_item(ui, NavIcon::Home, "Home", matches!(page, Page::Home)) {
                        acts.push(Action::Go(Page::Home));
                    }
                    if nav_item(ui, NavIcon::Charts, "Charts", matches!(page, Page::Charts)) {
                        acts.push(Action::Go(Page::Charts));
                    }
                    if nav_item(ui, NavIcon::Profile, "Profile", matches!(page, Page::Profile)) {
                        acts.push(Action::Go(Page::Profile));
                    }
                    if nav_item(ui, NavIcon::Avatar, "Avatar", matches!(page, Page::Avatar)) {
                        acts.push(Action::Go(Page::Avatar));
                    }
                    if nav_item(ui, NavIcon::Settings, "Settings", matches!(page, Page::Settings)) {
                        acts.push(Action::Go(Page::Settings));
                    }
                });
        }

        // Top bar
        egui::Panel::top("top")
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let logo_img = ui.add(
                        egui::Image::new(egui::include_image!("../assets/logo.png"))
                            .fit_to_exact_size(egui::vec2(30.0, 30.0))
                            .sense(if logged_in { Sense::click() } else { Sense::hover() }),
                    );
                    let logo_lbl = brand_title(ui, 21.0, logged_in);
                    if logged_in
                        && (logo_img
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                            || logo_lbl.clicked())
                    {
                        acts.push(Action::Go(Page::Home));
                    }

                    if logged_in {
                        ui.add_space(20.0);
                        let search_w = (ui.available_width() - 220.0).clamp(180.0, 440.0);
                        let search_pad = ((ui.available_width() - 200.0 - search_w) * 0.5).max(8.0);
                        ui.add_space(search_pad);

                        egui::Frame::new()
                            .fill(p.surface_bg)
                            .corner_radius(18)
                            .inner_margin(egui::Margin::symmetric(14, 6))
                            .show(ui, |ui| {
                                ui.set_width(search_w);
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::Image::new(egui::include_image!(
                                            "../assets/icon_search.png"
                                        ))
                                        .fit_to_exact_size(egui::vec2(14.0, 14.0))
                                        .tint(p.muted),
                                    );
                                    let edit_w = (search_w - 42.0).max(80.0);
                                    let r = ui.add(
                                        egui::TextEdit::singleline(&mut self.search)
                                            .frame(egui::Frame::NONE)
                                            .hint_text("Search")
                                            .desired_width(edit_w),
                                    );
                                    if r.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    {
                                        do_search = true;
                                    }
                                    if !self.search.is_empty()
                                        && ui
                                            .add(
                                                egui::Label::new(
                                                    RichText::new("âœ•").size(12.0).color(p.muted),
                                                )
                                                .sense(Sense::click()),
                                            )
                                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                                            .clicked()
                                    {
                                        self.search.clear();
                                    }
                                });
                            });
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if logged_in {
                            if let Some(prof) = &self.me {
                                let resp = ui
                                    .horizontal(|ui| {
                                        let n = ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 4.0;
                                            let l = ui.add(
                                                egui::Label::new(
                                                    RichText::new(&prof.user.name).size(13.0).strong(),
                                                )
                                                .sense(Sense::click()),
                                            );
                                            if prof.user.verified {
                                                verified_badge_tip(ui, 13.0, "Verified Account");
                                            }
                                            l
                                        })
                                        .inner;
                                        let h = thumb_styled(
                                            ui,
                                            Some(&prof.user.headshot),
                                            egui::vec2(28.0, 28.0),
                                            14,
                                            true,
                                        );
                                        (n.clicked() || h.clicked(), n, h)
                                    })
                                    .inner;
                                if resp.0 {
                                    acts.push(Action::Go(Page::Profile));
                                }
                                let mut menu = |ui: &mut egui::Ui| {
                                    if ui.button("Profile").clicked() {
                                        acts.push(Action::Go(Page::Profile));
                                        ui.close();
                                    }
                                    if ui.button("Avatar").clicked() {
                                        acts.push(Action::Go(Page::Avatar));
                                        ui.close();
                                    }
                                    if ui.button("Settings").clicked() {
                                        acts.push(Action::Go(Page::Settings));
                                        ui.close();
                                    }
                                    if ui
                                        .button(if self.settings.dark {
                                            "Switch to Light mode"
                                        } else {
                                            "Switch to Dark mode"
                                        })
                                        .clicked()
                                    {
                                        toggle_theme = true;
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button(RichText::new("Sign out").color(RED)).clicked() {
                                        acts.push(Action::SignOut);
                                        ui.close();
                                    }
                                };
                                resp.1.context_menu(&mut menu);
                                resp.2.context_menu(&mut menu);
                            }
                        } else {
                            let theme_icon = if self.settings.dark { "Light" } else { "Dark" };
                            let tbtn = egui::Button::new(
                                RichText::new(theme_icon).size(12.0).color(p.muted),
                            )
                            .fill(p.surface_bg)
                            .corner_radius(16);
                            if ui.add(tbtn).on_hover_text("Toggle theme").clicked() {
                                toggle_theme = true;
                            }
                        }
                    });
                });
            });

        // Status bar (only shown when there is an active message)
        if !self.status.is_empty() {
            egui::Panel::bottom("status")
                .resizable(false)
                .show_separator_line(false)
                .frame(
                    egui::Frame::new()
                        .fill(p.bg)
                        .inner_margin(egui::Margin::symmetric(20, 5)),
                )
                .show(ui, |ui| {
                    small_text(ui, self.status.clone());
                });
        }

        // Page content (central panel always last)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(egui::Margin::symmetric(22, 6)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if !logged_in {
                            self.page_login(ui, &mut acts);
                        } else {
                            match self.page.clone() {
                                Page::Home => self.page_home(ui, &mut acts),
                                Page::Charts => self.page_charts(ui, &mut acts),
                                Page::Search => self.page_search(ui, &mut acts),
                                Page::Game(g) => self.page_game(ui, &g, &mut acts),
                                Page::Friend(u) => self.page_friend(ui, &u, &mut acts),
                                Page::AddFriend => self.page_add_friend(ui, &mut acts),
                                Page::Profile => self.page_my_profile(ui, &mut acts),
                                Page::Avatar => self.page_avatar(ui, &mut acts),
                                Page::Settings => self.page_settings(ui, &mut acts),
                                Page::Login => self.page_login(ui, &mut acts),
                            }
                        }
                    });
            });

        if toggle_theme {
            self.settings.dark = !self.settings.dark;
            save_settings(&self.settings);
            apply_theme(ui.ctx(), self.settings.dark);
        }

        let hovered_game = acts.iter().find_map(|a| {
            if let Action::HoverGame(uid) = a {
                Some(*uid)
            } else {
                None
            }
        });
        self.handle_game_hover(hovered_game, ui.ctx());

        // Apply clicks after drawing
        for a in acts {
            match a {
                Action::HoverGame(_) => {}
                Action::StartQuickLogin => self.begin_quick_login(),
                Action::SignOut => self.sign_out(),
                Action::OpenUrl(u) => {
                    if let Err(e) = open::that(&u) {
                        self.status = format!("Error: {e}");
                    }
                }
                _ if !self.cookie.is_some() => {}
                Action::Go(p) => {
                    if !matches!(p, Page::Login) {
                        self.cancel_quick_login();
                    }
                    if matches!(p, Page::Avatar) {
                        self.ensure_avatar_loaded();
                    }
                    if matches!(p, Page::Profile) {
                        self.ensure_my_profile_loaded();
                    }
                    if matches!(p, Page::Home | Page::Charts) {
                        if Self::is_stale(
                            &self.last_sorts_sync,
                            &self.sorts_sync_started,
                            Duration::from_secs(12),
                        ) {
                            self.sync_sorts(false);
                        }
                        if Self::is_stale(
                            &self.last_saved_sync,
                            &self.saved_sync_started,
                            Duration::from_secs(12),
                        ) {
                            self.sync_saved(false);
                        }
                        if Self::is_stale(
                            &self.last_auth_sync,
                            &self.auth_sync_started,
                            Duration::from_secs(12),
                        ) {
                            self.sync_auth_session(false);
                        }
                    }
                    self.page = p;
                }
                Action::RefreshGames => self.force_refresh_games(),
                Action::RefreshMyProfile => {
                    self.my_profile = None;
                    self.my_profile_err = None;
                    self.ensure_my_profile_loaded();
                }
                Action::SaveBio(bio) => {
                    if let Some(cookie) = self.cookie.clone() {
                        self.profile_saving_bio = true;
                        self.profile_save_err = None;
                        let bio_clone = bio.clone();
                        self.spawn(move || {
                            let res = update_user_description(&cookie, &bio_clone);
                            Msg::BioSaved(res)
                        });
                    }
                }
                Action::SelectAvatarTab(tab) => {
                    self.avatar_tab = tab;
                    self.ensure_avatar_tab_loaded(tab);
                }
                Action::RefreshAvatar => {
                    self.load_avatar();
                    self.avatar_3d.loaded_user_id = None;
                    self.load_avatar_3d();
                    if let Some((uid, cookie)) = self
                        .me
                        .as_ref()
                        .map(|p| p.user.id)
                        .zip(self.cookie.clone())
                    {
                        self.avatar_seq += 1;
                        self.avatar_pending += 1;
                        let seq = self.avatar_seq;
                        self.spawn(move || {
                            Msg::AvatarApplied(
                                seq,
                                finish_avatar_update(
                                    uid,
                                    &cookie,
                                    "Refreshed avatar preview.".to_string(),
                                ),
                            )
                        });
                    }
                }
                Action::RefreshAvatarTab(tab) => {
                    match tab {
                        AvatarTab::Wearing | AvatarTab::Body => self.load_avatar(),
                        AvatarTab::Outfits => {
                            self.avatar_outfits = None;
                            self.avatar_outfits_page = 1;
                            self.avatar_outfits_more = false;
                            self.load_avatar_tab(tab, false);
                        }
                        t => {
                            self.avatar_items.remove(&t);
                            self.avatar_cursors.remove(&t);
                            self.load_avatar_tab(t, false);
                        }
                    }
                }
                Action::LoadMoreAvatarTab(tab) => {
                    self.load_avatar_tab(tab, true);
                }
                Action::ToggleWearAsset(item) => {
                    if let (Some(uid), Some(cookie)) =
                        (self.me.as_ref().map(|p| p.user.id), self.cookie.clone())
                    {
                        if let Some(st) = &mut self.avatar_state {
                            let (next, equipped) = compute_toggled_assets(&st.wearing, &item);
                            st.wearing = next.clone();
                            let note = if equipped {
                                format!("Equipped \"{}\".", item.name)
                            } else {
                                format!("Removed \"{}\".", item.name)
                            };
                            self.avatar_err = None;
                            self.avatar_seq += 1;
                            self.avatar_pending += 1;
                            let seq = self.avatar_seq;
                            self.status = if equipped {
                                format!("Equipped {} â€” syncing render...", item.name)
                            } else {
                                format!("Removed {} â€” syncing render...", item.name)
                            };
                            self.spawn(move || {
                                Msg::AvatarApplied(
                                    seq,
                                    apply_avatar_assets(uid, &cookie, &next, note),
                                )
                            });
                        }
                    }
                }
                Action::UnequipAccessories => {
                    if let (Some(uid), Some(cookie)) =
                        (self.me.as_ref().map(|p| p.user.id), self.cookie.clone())
                    {
                        if let Some(st) = &mut self.avatar_state {
                            let next: Vec<AvatarAsset> = st
                                .wearing
                                .iter()
                                .filter(|a| !matches!(a.asset_type_id, 8 | 41..=47))
                                .cloned()
                                .collect();
                            st.wearing = next.clone();
                            self.avatar_err = None;
                            self.avatar_seq += 1;
                            self.avatar_pending += 1;
                            let seq = self.avatar_seq;
                            self.status = "Removed hats & accessories â€” syncing render...".to_string();
                            self.spawn(move || {
                                Msg::AvatarApplied(
                                    seq,
                                    apply_avatar_assets(
                                        uid,
                                        &cookie,
                                        &next,
                                        "Removed all hats & accessories.".to_string(),
                                    ),
                                )
                            });
                        }
                    }
                }
                Action::WearOutfit(outfit) => {
                    if let (Some(uid), Some(cookie)) =
                        (self.me.as_ref().map(|p| p.user.id), self.cookie.clone())
                    {
                        self.avatar_err = None;
                        self.avatar_seq += 1;
                        self.avatar_pending += 1;
                        let seq = self.avatar_seq;
                        self.status = format!("Applying outfit \"{}\"...", outfit.name);
                        self.spawn(move || {
                            Msg::AvatarApplied(seq, apply_wear_outfit(uid, &cookie, &outfit))
                        });
                    }
                }
                Action::SetAvatarType(rig) => {
                    if let (Some(uid), Some(cookie)) =
                        (self.me.as_ref().map(|p| p.user.id), self.cookie.clone())
                    {
                        if let Some(st) = &mut self.avatar_state {
                            st.avatar_type = rig.clone();
                        }
                        self.avatar_err = None;
                        self.avatar_seq += 1;
                        self.avatar_pending += 1;
                        let seq = self.avatar_seq;
                        self.status = format!("Switched rig to {rig} â€” syncing render...");
                        self.spawn(move || {
                            Msg::AvatarApplied(seq, apply_avatar_type(uid, &cookie, &rig))
                        });
                    }
                }
                Action::SetBodyColorId(color_id) => {
                    if let (Some(uid), Some(cookie)) =
                        (self.me.as_ref().map(|p| p.user.id), self.cookie.clone())
                    {
                        if let Some(st) = &mut self.avatar_state {
                            let mut colors = st.colors;
                            match self.avatar_color_target {
                                BodyColorTarget::All => {
                                    colors = BodyColors {
                                        head: color_id,
                                        torso: color_id,
                                        right_arm: color_id,
                                        left_arm: color_id,
                                        right_leg: color_id,
                                        left_leg: color_id,
                                    };
                                }
                                BodyColorTarget::Head => colors.head = color_id,
                                BodyColorTarget::Torso => colors.torso = color_id,
                                BodyColorTarget::LeftArm => colors.left_arm = color_id,
                                BodyColorTarget::RightArm => colors.right_arm = color_id,
                                BodyColorTarget::LeftLeg => colors.left_leg = color_id,
                                BodyColorTarget::RightLeg => colors.right_leg = color_id,
                            }
                            st.colors = colors;
                            self.avatar_err = None;
                            self.avatar_seq += 1;
                            self.avatar_pending += 1;
                            let seq = self.avatar_seq;
                            self.status = "Updating skin tone...".to_string();
                            self.spawn(move || {
                                Msg::AvatarApplied(seq, apply_body_colors(uid, &cookie, colors))
                            });
                        }
                    }
                }
                Action::ApplyAvatarScales => {
                    if let (Some(st), Some(uid), Some(cookie)) = (
                        self.avatar_state.as_ref(),
                        self.me.as_ref().map(|p| p.user.id),
                        self.cookie.clone(),
                    ) {
                        let scales = st.scales;
                        self.avatar_err = None;
                        self.avatar_seq += 1;
                        self.avatar_pending += 1;
                        let seq = self.avatar_seq;
                        self.status = "Updating avatar scales...".to_string();
                        self.spawn(move || {
                            Msg::AvatarApplied(seq, apply_avatar_scales(uid, &cookie, scales))
                        });
                    }
                }
                Action::ResetAvatarScales => {
                    if let (Some(uid), Some(cookie)) =
                        (self.me.as_ref().map(|p| p.user.id), self.cookie.clone())
                    {
                        let scales = AvatarScales {
                            height: 1.0,
                            width: 1.0,
                            head: 1.0,
                            depth: 1.0,
                            proportion: 0.0,
                            body_type: 0.0,
                        };
                        if let Some(st) = &mut self.avatar_state {
                            st.scales = scales;
                        }
                        self.avatar_err = None;
                        self.avatar_seq += 1;
                        self.avatar_pending += 1;
                        let seq = self.avatar_seq;
                        self.status = "Resetting avatar scales...".to_string();
                        self.spawn(move || {
                            Msg::AvatarApplied(seq, apply_avatar_scales(uid, &cookie, scales))
                        });
                    }
                }
                Action::DismissAvatarErr => {
                    self.avatar_err = None;
                }
                Action::GoChart(i) => {
                    self.chart = i;
                    self.page = Page::Charts;
                    if Self::is_stale(
                        &self.last_sorts_sync,
                        &self.sorts_sync_started,
                        Duration::from_secs(12),
                    ) {
                        self.sync_sorts(false);
                    }
                }
                Action::Open(g) => self.open_game_details(g),
                Action::SelectGameTab(tab) => {
                    self.game_tab = tab;
                    if let Page::Game(g) = &self.page {
                        let uid = g.universe_id;
                        let pid = g.place_id;
                        let need_passes = tab == GameTab::Store
                            && !self
                                .game_extra
                                .as_ref()
                                .is_some_and(|e| e.universe_id == uid && e.passes_loaded);
                        let need_servers = tab == GameTab::Servers
                            && !self
                                .game_extra
                                .as_ref()
                                .is_some_and(|e| e.universe_id == uid && e.servers_loaded);
                        if need_passes {
                            self.load_game_passes(uid);
                        }
                        if need_servers {
                            self.load_game_servers(uid, pid, false);
                        }
                    }
                }
                Action::GameMediaStep(delta) => {
                    let len = self
                        .game_extra
                        .as_ref()
                        .map(|e| e.media.len())
                        .unwrap_or(0);
                    if len > 1 {
                        let n = len as isize;
                        self.game_media_idx = ((self.game_media_idx as isize + delta as isize)
                            .rem_euclid(n)) as usize;
                    }
                }
                Action::GameMediaSelect(idx) => {
                    self.game_media_idx = idx;
                }
                Action::JoinServer(pid, job_id) => {
                    self.status = "Getting launch ticket for server...".to_string();
                    self.record_recent_place_id(pid);
                    self.launch_server(pid, job_id);
                }
                Action::VoteGame(uid, vote) => {
                    if let Some(cookie) = self.cookie.clone() {
                        let prev = self
                            .game_extra
                            .as_ref()
                            .filter(|e| e.universe_id == uid)
                            .and_then(|e| e.user_vote);
                        if let Some(ex) = &mut self.game_extra {
                            if ex.universe_id == uid {
                                ex.user_vote = vote;
                            }
                        }
                        if let Page::Game(g) = &mut self.page {
                            if g.universe_id == uid {
                                match (prev, vote) {
                                    (Some(true), Some(false)) => {
                                        g.up = g.up.saturating_sub(1);
                                        g.down = g.down.saturating_add(1);
                                    }
                                    (Some(false), Some(true)) => {
                                        g.down = g.down.saturating_sub(1);
                                        g.up = g.up.saturating_add(1);
                                    }
                                    (None, Some(true)) => g.up = g.up.saturating_add(1),
                                    (None, Some(false)) => g.down = g.down.saturating_add(1),
                                    (Some(true), None) => g.up = g.up.saturating_sub(1),
                                    (Some(false), None) => g.down = g.down.saturating_sub(1),
                                    _ => {}
                                }
                            }
                        }
                        self.spawn(move || {
                            Msg::GameVoteDone(uid, vote, vote_game(uid, vote, &cookie))
                        });
                    }
                }
                Action::RefreshServers(uid, pid) => {
                    if let Some(ex) = &mut self.game_extra {
                        if ex.universe_id == uid {
                            ex.servers.clear();
                            ex.servers_cursor = None;
                        }
                    }
                    self.load_game_servers(uid, pid, false);
                }
                Action::LoadMoreServers(uid, pid) => {
                    self.load_game_servers(uid, pid, true);
                }
                Action::OpenAddFriend => {
                    if !matches!(self.page, Page::AddFriend) {
                        self.back = self.page.clone();
                    }
                    self.page = Page::AddFriend;
                }
                Action::SearchUsers(q) => {
                    let q = q.trim().to_string();
                    if !q.is_empty() {
                        let cookie = self.cookie.clone();
                        let sid = self.sid.clone();
                        self.friend_searched = q.clone();
                        self.friend_searching = true;
                        self.friend_search_loading_more = false;
                        self.friend_search_cursor = None;
                        self.friend_search_results = None;
                        self.friend_search_verified.clear();
                        self.spawn(move || {
                            Msg::FriendSearchResult(
                                false,
                                search_users_api(&q, None, &sid, cookie.as_deref()),
                            )
                        });
                    }
                }
                Action::LoadMoreUsers => {
                    if !self.friend_search_loading_more && self.friend_search_cursor.is_some() {
                        let q = self.friend_searched.clone();
                        let cursor = self.friend_search_cursor.clone();
                        let cookie = self.cookie.clone();
                        let sid = self.sid.clone();
                        self.friend_search_loading_more = true;
                        self.spawn(move || {
                            Msg::FriendSearchResult(
                                true,
                                search_users_api(
                                    &q,
                                    cursor.as_deref(),
                                    &sid,
                                    cookie.as_deref(),
                                ),
                            )
                        });
                    }
                }
                Action::SendFriendRequest(uid, uname) => {
                    self.friend_sent.insert(uid);
                    if let Some(cookie) = self.cookie.clone() {
                        self.status = format!("Sending friend request to @{uname}...");
                        self.spawn(move || {
                            Msg::FriendRequestDone(
                                uid,
                                uname,
                                send_friend_request_api(uid, &cookie),
                            )
                        });
                    }
                }
                Action::OpenFriend(user) => {
                    if !matches!(self.page, Page::Friend(_)) {
                        self.back = self.page.clone();
                    }
                    let uid = user.id;
                    let cookie = self.cookie.clone();
                    self.friend_profile = None;
                    self.friend_err = None;
                    self.friend_loading = true;
                    self.page = Page::Friend(user.clone());
                    self.spawn(move || Msg::FriendProfile(uid, fetch_friend_profile(user, cookie)));
                }
                Action::Play(g) => self.play(&g),
                Action::JoinPlace(pid) => {
                    self.status = "Joining friend...".to_string();
                    self.record_recent_place_id(pid);
                    self.launch_place_id(pid);
                }
                Action::ToggleFav(g) => self.toggle_fav(&g),
            }
        }
        if do_search && self.cookie.is_some() {
            self.start_search();
        }
    }

    fn on_exit(&mut self) {
        self.watcher_stop.store(true, Ordering::Relaxed);
        self.auto_close_flag.store(false, Ordering::Relaxed);
        if self.settings.auto_close_roblox {
            close_tray_roblox_processes();
        }
    }
}

