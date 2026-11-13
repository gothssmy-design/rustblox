use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use eframe::egui::{self, Color32};
use serde_json::Value;

use crate::types::{
    AvatarAsset, AvatarScales, AvatarState, AvatarTab, BodyColors, FriendProfile, Game, GameExtra,
    GamePassItem, Msg, OmniHome, OutfitItem, PaletteColor, Profile, SavedSyncData, ServerItem,
    Sort, User, UserSearchPage,
};
use crate::video_preview;

pub fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(15))
            .user_agent("rustblox/0.1")
            .build()
    })
}

pub fn get_json(url: &str) -> Result<Value, String> {
    auth_get_json(url, None)
}

pub fn auth_get_json(url: &str, cookie: Option<&str>) -> Result<Value, String> {
    let mut req = agent().get(url);
    if let Some(c) = cookie {
        req = req.set("Cookie", &format!(".ROBLOSECURITY={c}"));
    }
    req.call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())
}

pub fn post_json(url: &str, body: Value) -> Result<Value, String> {
    agent()
        .post(url)
        .send_json(body)
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())
}

pub fn build_json_req(
    method: &str,
    url: &str,
    cookie: Option<&str>,
    referer: Option<&str>,
    csrf: Option<&str>,
) -> ureq::Request {
    let mut req = agent()
        .request(method, url)
        .set("Content-Type", "application/json")
        .set("Accept", "application/json");
    if let Some(c) = cookie {
        req = req.set("Cookie", &format!(".ROBLOSECURITY={c}"));
    }
    if let Some(r) = referer {
        req = req.set("Referer", r);
    }
    if let Some(t) = csrf {
        req = req.set("x-csrf-token", t);
    }
    req
}

pub fn parse_ureq_err(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, resp) => {
            if let Ok(v) = resp.into_json::<Value>() {
                if let Some(msg) = v["errors"][0]["message"].as_str() {
                    return msg.to_string();
                }
                if let Some(msg) = v["message"].as_str() {
                    return msg.to_string();
                }
            }
            format!("HTTP {code}")
        }
        e => e.to_string(),
    }
}

pub fn send_csrf(
    method: &str,
    url: &str,
    body: Value,
    cookie: Option<&str>,
    referer: Option<&str>,
) -> Result<ureq::Response, String> {
    match build_json_req(method, url, cookie, referer, None).send_json(body.clone()) {
        Ok(resp) => Ok(resp),
        Err(ureq::Error::Status(403, resp)) => {
            if let Some(token) = resp.header("x-csrf-token").map(str::to_string) {
                build_json_req(method, url, cookie, referer, Some(&token))
                    .send_json(body)
                    .map_err(parse_ureq_err)
            } else {
                Err("403 Forbidden (missing CSRF token)".to_string())
            }
        }
        Err(e) => Err(parse_ureq_err(e)),
    }
}

pub fn post_csrf(
    url: &str,
    body: Value,
    cookie: Option<&str>,
    referer: Option<&str>,
) -> Result<ureq::Response, String> {
    send_csrf("POST", url, body, cookie, referer)
}

pub fn patch_csrf(
    url: &str,
    body: Value,
    cookie: Option<&str>,
    referer: Option<&str>,
) -> Result<ureq::Response, String> {
    send_csrf("PATCH", url, body, cookie, referer)
}

pub fn extract_roblosecurity(resp: &ureq::Response) -> Option<String> {
    for header in resp.all("set-cookie") {
        if let Some(rest) = header.trim_start().strip_prefix(".ROBLOSECURITY=") {
            let value = rest.split(';').next().unwrap_or("").trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn join(ids: &[u64]) -> String {
    ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",")
}

pub fn format_iso_date(iso: &str) -> String {
    let date = iso.split('T').next().unwrap_or("");
    let mut parts = date.split('-');
    if let (Some(y), Some(m), Some(d)) = (parts.next(), parts.next(), parts.next()) {
        if let (Ok(yi), Ok(mi), Ok(di)) = (y.parse::<u32>(), m.parse::<u32>(), d.parse::<u32>()) {
            return format!("{mi}/{di}/{yi}");
        }
    }
    "--".to_string()
}

pub fn format_commas(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

pub fn str_of(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

pub fn game_from_explore(v: &Value) -> Option<Game> {
    if v["isSponsored"].as_bool() == Some(true) {
        return None;
    }
    Some(Game {
        universe_id: v["universeId"].as_u64()?,
        place_id: v["rootPlaceId"].as_u64()?,
        name: v["name"].as_str()?.to_string(),
        creator: str_of(&v["creatorName"]),
        creator_verified: v["creatorHasVerifiedBadge"].as_bool().unwrap_or(false),
        playing: v["playerCount"].as_u64().unwrap_or(0),
        up: v["totalUpVotes"].as_u64().unwrap_or(0),
        down: v["totalDownVotes"].as_u64().unwrap_or(0),
        description: str_of(&v["description"]),
    })
}

static SID_SEQ: AtomicU64 = AtomicU64::new(1);

fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

pub fn new_session_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = SID_SEQ.fetch_add(1, Ordering::Relaxed);
    let a = mix64((nanos as u64) ^ seq.wrapping_mul(0x517cc1b727220a95));
    let b = mix64(((nanos >> 64) as u64) ^ seq ^ 0xa0761d6478bd642f);
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&a.to_le_bytes());
    bytes[8..].copy_from_slice(&b.to_le_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

pub fn val_u64(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
}

pub fn fetch_omni_home(sid: &str, cookie: &str) -> Result<OmniHome, String> {
    let body = serde_json::json!({
        "pageType": "Home",
        "sessionId": sid,
        "supportedTreatmentTypes": ["SortlessGrid", "Carousel"],
    });
    let resp = post_csrf(
        "https://apis.roblox.com/discovery-api/omni-recommendation",
        body,
        Some(cookie),
        Some("https://www.roblox.com/"),
    )?;
    let v: Value = resp.into_json().map_err(|e| e.to_string())?;
    let mut continue_unis = None;
    let mut topics = Vec::new();
    for s in v["sorts"].as_array().into_iter().flatten() {
        let topic = str_of(&s["topic"]);
        let treatment = str_of(&s["treatmentType"]);
        let topic_id = val_u64(&s["topicId"]).unwrap_or(0);
        let lower_topic = topic.to_ascii_lowercase();
        if treatment.eq_ignore_ascii_case("Sponsored") || lower_topic.contains("sponsored") {
            continue;
        }
        let mut row_unis = Vec::new();
        let mut seen = HashSet::new();
        for item in s["recommendationList"].as_array().into_iter().flatten() {
            if item["contentType"].as_str() != Some("Game") {
                continue;
            }
            if item["contentMetadata"]["isSponsored"].as_bool() == Some(true) {
                continue;
            }
            if let Some(uid) = val_u64(&item["contentId"]) {
                if uid > 0 && seen.insert(uid) {
                    row_unis.push(uid);
                    if row_unis.len() >= 24 {
                        break;
                    }
                }
            }
        }
        let is_continue = topic.eq_ignore_ascii_case("Continue") || topic_id == 100000003;
        if is_continue {
            if continue_unis.is_none() {
                continue_unis = Some(row_unis);
            }
        } else if !topic.is_empty() && !row_unis.is_empty() {
            topics.push((topic, row_unis));
        }
    }
    Ok(OmniHome {
        continue_unis,
        topics,
    })
}

pub fn fetch_sorts(sid: &str, cookie: Option<&str>) -> Result<Vec<Sort>, String> {
    let mut out = Vec::new();
    let mut token = String::new();
    let mut explore_err = None;
    for page in 0..3 {
        let mut url = format!(
            "https://apis.roblox.com/explore-api/v1/get-sorts?sessionId={sid}&device=computer&country=all"
        );
        if !token.is_empty() {
            url.push_str(&format!("&sortsPageToken={}", enc(&token)));
        }
        let v = match auth_get_json(&url, cookie) {
            Ok(v) => v,
            Err(e) if page == 0 => {
                explore_err = Some(e);
                break;
            }
            Err(_) => break,
        };
        for s in v["sorts"].as_array().into_iter().flatten() {
            if s["contentType"].as_str() != Some("Games") {
                continue;
            }
            let games: Vec<Game> = s["games"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(game_from_explore)
                .collect();
            let title = str_of(&s["sortDisplayName"]);
            if !games.is_empty()
                && !out
                    .iter()
                    .any(|x: &Sort| x.title.eq_ignore_ascii_case(&title))
            {
                out.push(Sort { title, games });
            }
        }
        token = str_of(&v["nextSortsPageToken"]);
        if token.is_empty() {
            break;
        }
    }

    if let Some(c) = cookie {
        if let Ok(omni) = fetch_omni_home(sid, c) {
            let mut extra_topics = Vec::new();
            let mut needed_ids = Vec::new();
            let mut seen_ids = HashSet::new();
            for (title, unis) in omni.topics {
                if out
                    .iter()
                    .any(|x: &Sort| x.title.eq_ignore_ascii_case(&title))
                    || extra_topics
                        .iter()
                        .any(|(t, _): &(String, Vec<u64>)| t.eq_ignore_ascii_case(&title))
                {
                    continue;
                }
                for &uid in &unis {
                    if seen_ids.insert(uid) {
                        needed_ids.push(uid);
                    }
                }
                extra_topics.push((title, unis));
            }
            if !needed_ids.is_empty() {
                let by_uid: HashMap<u64, Game> = games_by_universe(&needed_ids)
                    .into_iter()
                    .map(|g| (g.universe_id, g))
                    .collect();
                for (title, unis) in extra_topics {
                    let games: Vec<Game> =
                        unis.into_iter().filter_map(|u| by_uid.get(&u).cloned()).collect();
                    if !games.is_empty() {
                        out.push(Sort { title, games });
                    }
                }
            }
        }
    }

    if out.is_empty() {
        if let Some(e) = explore_err {
            return Err(e);
        }
    }
    Ok(out)
}

pub fn search_games(q: &str, sid: &str) -> Result<Vec<Game>, String> {
    let v = get_json(&format!(
        "https://apis.roblox.com/search-api/omni-search?searchQuery={}&sessionId={sid}&pageType=all",
        enc(q)
    ))?;
    let mut out = Vec::new();
    for group in v["searchResults"].as_array().into_iter().flatten() {
        if group["contentGroupType"].as_str() != Some("Game") {
            continue;
        }
        out.extend(
            group["contents"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(game_from_explore),
        );
    }
    Ok(out)
}

pub fn place_universe_cache() -> &'static Mutex<HashMap<u64, u64>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, u64>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn remember_place_universe(place_id: u64, universe_id: u64) {
    if place_id > 0 && universe_id > 0 {
        if let Ok(mut map) = place_universe_cache().lock() {
            map.insert(place_id, universe_id);
        }
    }
}

pub fn cached_universe_of_place(place_id: u64) -> Option<u64> {
    place_universe_cache()
        .lock()
        .ok()
        .and_then(|map| map.get(&place_id).copied())
}

pub fn games_by_universe(ids: &[u64]) -> Vec<Game> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for &id in ids {
        if id > 0 && seen.insert(id) {
            unique.push(id);
        }
    }
    let mut by_id: HashMap<u64, Game> = HashMap::new();
    for chunk in unique.chunks(50) {
        let list = join(chunk);
        if let Ok(v) = get_json(&format!("https://games.roblox.com/v1/games?universeIds={list}")) {
            for g in v["data"].as_array().into_iter().flatten() {
                let Some(id) = g["id"].as_u64() else { continue };
                let place_id = g["rootPlaceId"].as_u64().unwrap_or(0);
                remember_place_universe(place_id, id);
                by_id.insert(
                    id,
                    Game {
                        universe_id: id,
                        place_id,
                        name: str_of(&g["name"]),
                        creator: str_of(&g["creator"]["name"]),
                        creator_verified: g["creator"]["hasVerifiedBadge"].as_bool().unwrap_or(false),
                        playing: g["playing"].as_u64().unwrap_or(0),
                        up: 0,
                        down: 0,
                        description: str_of(&g["description"]),
                    },
                );
            }
        }
        if let Ok(v) = get_json(&format!(
            "https://games.roblox.com/v1/games/votes?universeIds={list}"
        )) {
            for x in v["data"].as_array().into_iter().flatten() {
                if let Some(game) = x["id"].as_u64().and_then(|id| by_id.get_mut(&id)) {
                    game.up = x["upVotes"].as_u64().unwrap_or(0);
                    game.down = x["downVotes"].as_u64().unwrap_or(0);
                }
            }
        }
    }
    ids.iter().filter_map(|id| by_id.get(id).cloned()).collect()
}

pub fn universe_of_place(place: u64) -> Option<u64> {
    if place == 0 {
        return None;
    }
    if let Some(uid) = cached_universe_of_place(place) {
        return Some(uid);
    }
    let uid = get_json(&format!(
        "https://apis.roblox.com/universes/v1/places/{place}/universe"
    ))
    .ok()?["universeId"]
        .as_u64()?;
    remember_place_universe(place, uid);
    Some(uid)
}

pub fn games_by_places(places: &[u64]) -> Vec<Game> {
    let unis: Vec<u64> = places.iter().filter_map(|p| universe_of_place(*p)).collect();
    games_by_universe(&unis)
}

pub fn fetch_user_favorite_games(uid: u64, cookie: &str) -> Result<Vec<u64>, String> {
    let v = auth_get_json(
        &format!("https://games.roblox.com/v2/users/{uid}/favorite/games?sortOrder=Desc&limit=25"),
        Some(cookie),
    )?;
    let mut unis = Vec::new();
    let mut seen = HashSet::new();
    for item in v["data"].as_array().into_iter().flatten() {
        if let Some(id) = item["id"].as_u64() {
            if let Some(pid) = item["rootPlace"]["id"].as_u64() {
                remember_place_universe(pid, id);
            }
            if id > 0 && seen.insert(id) {
                unis.push(id);
            }
        }
    }
    Ok(unis)
}

pub fn fetch_saved_synced(
    cookie: &str,
    uid_opt: Option<u64>,
    local_recent_places: Vec<u64>,
    local_fav_places: Vec<u64>,
) -> SavedSyncData {
    let uid = uid_opt.or_else(|| {
        auth_get_json("https://users.roblox.com/v1/users/authenticated", Some(cookie))
            .ok()
            .and_then(|v| v["id"].as_u64())
    });

    let omni_continue = fetch_omni_home(&new_session_id(), cookie)
        .ok()
        .and_then(|o| o.continue_unis);
    let remote_fav_unis = uid.and_then(|id| fetch_user_favorite_games(id, cookie).ok());

    let local_recent_unis: Vec<u64> = if omni_continue.as_ref().is_some_and(|v| !v.is_empty()) {
        local_recent_places
            .iter()
            .take(8)
            .filter_map(|p| universe_of_place(*p))
            .collect()
    } else {
        local_recent_places
            .iter()
            .filter_map(|p| universe_of_place(*p))
            .collect()
    };

    let local_fav_unis: Vec<u64> = if remote_fav_unis.is_none() {
        local_fav_places
            .iter()
            .filter_map(|p| universe_of_place(*p))
            .collect()
    } else {
        Vec::new()
    };

    let mut all_unis = Vec::new();
    let mut seen = HashSet::new();
    for &u in omni_continue
        .iter()
        .flatten()
        .chain(remote_fav_unis.iter().flatten())
        .chain(local_recent_unis.iter())
        .chain(local_fav_unis.iter())
    {
        if u > 0 && seen.insert(u) {
            all_unis.push(u);
        }
    }

    let by_uid: HashMap<u64, Game> = if all_unis.is_empty() {
        HashMap::new()
    } else {
        games_by_universe(&all_unis)
            .into_iter()
            .map(|g| (g.universe_id, g))
            .collect()
    };

    let resolve_list = |unis: &[u64]| -> Option<Vec<Game>> {
        let resolved: Vec<Game> = unis.iter().filter_map(|u| by_uid.get(u).cloned()).collect();
        if !unis.is_empty() && resolved.is_empty() {
            None
        } else {
            Some(resolved)
        }
    };

    let remote_recent = omni_continue.as_deref().and_then(resolve_list);
    let fallback_recent = if local_recent_unis.is_empty() {
        None
    } else {
        resolve_list(&local_recent_unis)
    };
    let remote_favs = remote_fav_unis.as_deref().and_then(resolve_list);
    let fallback_favs = if remote_fav_unis.is_none() && !local_fav_unis.is_empty() {
        resolve_list(&local_fav_unis)
    } else {
        None
    };

    SavedSyncData {
        remote_recent,
        fallback_recent,
        remote_favs,
        fallback_favs,
    }
}

pub fn fetch_game_extra(universe_id: u64, cookie: Option<&str>) -> GameExtra {
    let mut ex = GameExtra {
        universe_id,
        verified_creator: false,
        favorites: 0,
        visits: 0,
        max_players: 0,
        created: "--".to_string(),
        updated: "--".to_string(),
        genre: "All".to_string(),
        subgenre: String::new(),
        maturity_short: String::new(),
        maturity_detail: String::new(),
        voice_chat: None,
        camera: None,
        is_favorited_api: None,
        can_vote: false,
        user_vote: None,
        media: Vec::new(),
        preview_video_id: None,
        passes: Vec::new(),
        passes_loaded: false,
        servers: Vec::new(),
        servers_cursor: None,
        servers_loaded: false,
    };

    if let Ok(v) = auth_get_json(
        &format!("https://games.roblox.com/v1/games?universeIds={universe_id}"),
        cookie,
    ) {
        let g = &v["data"][0];
        if g.is_object() {
            ex.verified_creator = g["creator"]["hasVerifiedBadge"].as_bool().unwrap_or(false);
            ex.favorites = g["favoritedCount"].as_u64().unwrap_or(0);
            ex.visits = g["visits"].as_u64().unwrap_or(0);
            ex.max_players = g["maxPlayers"].as_u64().unwrap_or(0);
            ex.created = format_iso_date(&str_of(&g["created"]));
            ex.updated = format_iso_date(&str_of(&g["updated"]));
            let gl1 = str_of(&g["genre_l1"]);
            let gl2 = str_of(&g["genre_l2"]);
            let fallback_g = str_of(&g["genre"]);
            ex.genre = if !gl1.is_empty() {
                gl1
            } else if !fallback_g.is_empty() {
                fallback_g
            } else {
                "All".to_string()
            };
            ex.subgenre = gl2;
            ex.is_favorited_api = g["isFavoritedByUser"].as_bool();
        }
    }

    if let Ok(v) = post_json(
        "https://apis.roblox.com/experience-guidelines-api/experience-guidelines/get-age-recommendation",
        serde_json::json!({ "universeId": universe_id }),
    ) {
        let age = &v["ageRecommendationDetails"]["summary"]["ageRecommendation"];
        let short = str_of(&age["displayNameWithHeaderShort"]);
        let name = str_of(&age["displayName"]);
        ex.maturity_short = if !short.is_empty() {
            short
        } else if !name.is_empty() {
            format!("Maturity: {name}")
        } else {
            String::new()
        };
        let descriptors: Vec<String> = v["ageRecommendationDetails"]["descriptorUsages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|d| d["contains"].as_bool() == Some(true))
            .filter_map(|d| {
                let s = str_of(&d["descriptor"]["displayName"]);
                (!s.is_empty()).then_some(s)
            })
            .collect();
        ex.maturity_detail = descriptors.join(", ");
    }

    if let Some(c) = cookie {
        if let Ok(v) = auth_get_json(
            &format!("https://voice.roblox.com/v1/settings/universe/{universe_id}"),
            Some(c),
        ) {
            ex.voice_chat = v["isUniverseEnabledForVoice"].as_bool();
            ex.camera = v["isUniverseEnabledForCamera"].as_bool();
        }
        if let Ok(v) = auth_get_json(
            &format!("https://games.roblox.com/v1/games/{universe_id}/votes/user"),
            Some(c),
        ) {
            ex.can_vote = v["canVote"].as_bool().unwrap_or(false);
            ex.user_vote = v["userVote"].as_bool();
        }
    }

    if let Ok(v) = get_json(&format!(
        "https://games.roblox.com/v2/games/{universe_id}/media"
    )) {
        for m in v["data"].as_array().into_iter().flatten() {
            if m["approved"].as_bool() != Some(false) {
                let is_video = m["assetTypeId"].as_u64() == Some(86)
                    || m["assetType"].as_str() == Some("GamePreviewVideo");
                if is_video && ex.preview_video_id.is_none() {
                    ex.preview_video_id = m["videoId"].as_u64().or_else(|| {
                        m["videoId"].as_str().and_then(|s| s.parse::<u64>().ok())
                    });
                }
            }
        }
        let img_ids: Vec<u64> = v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|m| m["approved"].as_bool() != Some(false))
            .filter_map(|m| m["imageId"].as_u64())
            .take(8)
            .collect();
        if !img_ids.is_empty() {
            if let Ok(t) = get_json(&format!(
                "https://thumbnails.roblox.com/v1/assets?assetIds={}&size=768x432&format=Png&isCircular=false",
                join(&img_ids)
            )) {
                let by_id: HashMap<u64, String> = t["data"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|d| {
                        let id = d["targetId"].as_u64()?;
                        let url = str_of(&d["imageUrl"]);
                        (!url.is_empty()).then_some((id, url))
                    })
                    .collect();
                for id in img_ids {
                    if let Some(url) = by_id.get(&id) {
                        ex.media.push(url.clone());
                    }
                }
            }
        }
    }

    ex
}

pub fn fetch_game_preview_video_id(universe_id: u64) -> Option<u64> {
    let v = get_json(&format!(
        "https://games.roblox.com/v2/games/{universe_id}/media"
    )).ok()?;
    for m in v["data"].as_array().into_iter().flatten() {
        if m["approved"].as_bool() != Some(false) {
            let is_video = m["assetTypeId"].as_u64() == Some(86)
                || m["assetType"].as_str() == Some("GamePreviewVideo");
            if is_video {
                if let Some(vid) = m["videoId"].as_u64().or_else(|| {
                    m["videoId"].as_str().and_then(|s| s.parse::<u64>().ok())
                }) {
                    if vid > 0 {
                        return Some(vid);
                    }
                }
            }
        }
    }
    None
}

pub fn fetch_and_decode_preview_video(
    video_id: u64,
    max_width: usize,
    cookie: Option<&str>,
) -> Result<video_preview::DecodedPreviewVideo, String> {
    // 1. Try v2/assetId endpoint which provides the signed CDN location URL
    let meta_url = format!("https://assetdelivery.roblox.com/v2/assetId/{video_id}");
    let direct_url = if let Ok(v2_json) = auth_get_json(&meta_url, cookie) {
        v2_json["locations"][0]["location"]
            .as_str()
            .map(str::to_string)
    } else {
        None
    };

    let target_url = direct_url.unwrap_or_else(|| {
        format!("https://assetdelivery.roblox.com/v1/asset/?id={video_id}")
    });

    let resp = agent()
        .get(&target_url)
        .call()
        .map_err(|e| format!("Download video error: {e}"))?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Read video error: {e}"))?;
    video_preview::decode_webm_vp8(&bytes, max_width)
}

pub fn fetch_game_passes(universe_id: u64, cookie: Option<&str>) -> Result<Vec<GamePassItem>, String> {
    let v = auth_get_json(
        &format!(
            "https://apis.roblox.com/game-passes/v1/universes/{universe_id}/game-passes?passView=Full&pageSize=50"
        ),
        cookie,
    )?;
    let mut icon_ids = Vec::new();
    let mut raw: Vec<(GamePassItem, u64)> = v["gamePasses"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|gp| {
            let id = gp["id"].as_u64()?;
            let name = {
                let dn = str_of(&gp["displayName"]);
                if dn.is_empty() {
                    str_of(&gp["name"])
                } else {
                    dn
                }
            };
            let description = str_of(&gp["displayDescription"]);
            let price = gp["price"].as_u64();
            let is_for_sale = gp["isForSale"].as_bool().unwrap_or(false);
            let is_owned = gp["isOwned"].as_bool().unwrap_or(false);
            let icon_id = gp["displayIconImageAssetId"].as_u64().unwrap_or(0);
            if icon_id > 0 {
                icon_ids.push(icon_id);
            }
            Some((
                GamePassItem {
                    id,
                    name,
                    description,
                    price,
                    is_for_sale,
                    is_owned,
                    thumb: String::new(),
                },
                icon_id,
            ))
        })
        .collect();
    let thumbs = fetch_asset_thumbs(&icon_ids);
    let mut passes = Vec::with_capacity(raw.len());
    for (mut item, icon_id) in raw.drain(..) {
        if let Some(url) = thumbs.get(&icon_id) {
            item.thumb = url.clone();
        }
        passes.push(item);
    }
    Ok(passes)
}

pub fn fetch_game_servers(
    place_id: u64,
    cursor: Option<&str>,
    cookie: Option<&str>,
) -> Result<(Vec<ServerItem>, Option<String>), String> {
    let mut url = format!(
        "https://games.roblox.com/v1/games/{place_id}/servers/Public?sortOrder=Desc&limit=25"
    );
    if let Some(c) = cursor.filter(|s| !s.is_empty()) {
        url.push_str("&cursor=");
        url.push_str(&enc(c));
    }
    let v = auth_get_json(&url, cookie)?;
    let next = v["nextPageCursor"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let servers: Vec<ServerItem> = v["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let id = s["id"].as_str()?.to_string();
            Some(ServerItem {
                id,
                playing: s["playing"].as_u64().unwrap_or(0),
                max_players: s["maxPlayers"].as_u64().unwrap_or(0),
                ping: s["ping"].as_u64(),
                fps: s["fps"].as_f64(),
            })
        })
        .collect();
    Ok((servers, next))
}

pub fn vote_game(universe_id: u64, vote: Option<bool>, cookie: &str) -> Result<(), String> {
    patch_csrf(
        &format!("https://games.roblox.com/v1/games/{universe_id}/user-votes"),
        serde_json::json!({ "vote": vote }),
        Some(cookie),
        Some("https://www.roblox.com/"),
    )?;
    Ok(())
}

pub fn favorite_game_api(universe_id: u64, is_fav: bool, cookie: &str) -> Result<(), String> {
    post_csrf(
        &format!("https://games.roblox.com/v1/games/{universe_id}/favorites"),
        serde_json::json!({ "isFavorited": is_fav }),
        Some(cookie),
        Some("https://www.roblox.com/"),
    )?;
    Ok(())
}

pub fn fetch_thumbs(ids: Vec<u64>) -> Msg {
    let mut wide = HashMap::new();
    let mut icon = HashMap::new();
    for chunk in ids.chunks(50) {
        let list = join(chunk);
        if let Ok(v) = get_json(&format!(
            "https://thumbnails.roblox.com/v1/games/multiget/thumbnails?universeIds={list}&countPerUniverse=1&defaults=true&size=768x432&format=Png&isCircular=false"
        )) {
            for d in v["data"].as_array().into_iter().flatten() {
                if let (Some(id), Some(url)) = (
                    d["universeId"].as_u64(),
                    d["thumbnails"][0]["imageUrl"].as_str(),
                ) {
                    wide.insert(id, url.to_string());
                }
            }
        }
        if let Ok(v) = get_json(&format!(
            "https://thumbnails.roblox.com/v1/games/icons?universeIds={list}&returnPolicy=PlaceHolder&size=256x256&format=Png&isCircular=false"
        )) {
            for d in v["data"].as_array().into_iter().flatten() {
                if let (Some(id), Some(url)) = (d["targetId"].as_u64(), d["imageUrl"].as_str()) {
                    icon.insert(id, url.to_string());
                }
            }
        }
    }
    Msg::Thumbs { wide, icon }
}

pub fn enrich_users(users: &mut [User], cookie: Option<&str>) {
    if users.is_empty() {
        return;
    }
    for chunk in users.chunks_mut(50) {
        let ids: Vec<u64> = chunk.iter().map(|u| u.id).collect();
        if !ids.is_empty() {
            if let Ok(v) = post_json(
                "https://users.roblox.com/v1/users",
                serde_json::json!({ "userIds": ids, "excludeBannedUsers": false }),
            ) {
                for item in v["data"].as_array().into_iter().flatten() {
                    let Some(uid) = item["id"].as_u64() else { continue };
                    if let Some(u) = chunk.iter_mut().find(|u| u.id == uid) {
                        let name = str_of(&item["name"]);
                        let disp = str_of(&item["displayName"]);
                        if u.name.is_empty() && !name.is_empty() {
                            u.name = name;
                        }
                        if u.display.is_empty() {
                            u.display = if disp.is_empty() { u.name.clone() } else { disp };
                        }
                        u.verified = item["hasVerifiedBadge"].as_bool().unwrap_or(false);
                    }
                }
            }
        }
        if let Ok(h) = get_json(&format!(
            "https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds={}&size=150x150&format=Png&isCircular=false",
            join(&ids)
        )) {
            let map: HashMap<u64, String> = h["data"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|d| Some((d["targetId"].as_u64()?, str_of(&d["imageUrl"]))))
                .collect();
            for u in chunk.iter_mut() {
                u.headshot = map.get(&u.id).cloned().unwrap_or_default();
            }
        }
        if let Some(c) = cookie {
            if let Ok(resp) = post_csrf(
                "https://presence.roblox.com/v1/presence/users",
                serde_json::json!({ "userIds": ids }),
                Some(c),
                None,
            ) {
                if let Ok(p) = resp.into_json::<Value>() {
                    for item in p["userPresences"].as_array().into_iter().flatten() {
                        let Some(uid) = item["userId"].as_u64() else { continue };
                        if let Some(u) = chunk.iter_mut().find(|u| u.id == uid) {
                            let kind = item["userPresenceType"].as_u64().unwrap_or(0) as u8;
                            u.presence = if kind == 4 { 0 } else { kind };
                            u.location = str_of(&item["lastLocation"]);
                            u.place_id = item["placeId"].as_u64().or_else(|| item["rootPlaceId"].as_u64());
                        }
                    }
                }
            }
        }
    }
}

pub fn fetch_users_by_ids(ids: &[u64], cookie: Option<&str>) -> Result<Vec<User>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut users = Vec::new();
    for chunk in ids.chunks(100) {
        let v = post_json(
            "https://users.roblox.com/v1/users",
            serde_json::json!({ "userIds": chunk, "excludeBannedUsers": true }),
        )?;
        for u in v["data"].as_array().into_iter().flatten() {
            if let Some(id) = u["id"].as_u64() {
                users.push(User {
                    id,
                    name: str_of(&u["name"]),
                    display: str_of(&u["displayName"]),
                    headshot: String::new(),
                    presence: 0,
                    location: String::new(),
                    place_id: None,
                    verified: u["hasVerifiedBadge"].as_bool().unwrap_or(false),
                });
            }
        }
    }
    enrich_users(&mut users, cookie);
    Ok(users)
}

pub fn search_users_api(
    query: &str,
    cursor: Option<&str>,
    sid: &str,
    cookie: Option<&str>,
) -> Result<UserSearchPage, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(UserSearchPage {
            users: Vec::new(),
            verified: HashSet::new(),
            next_cursor: None,
        });
    }

    let mut users: Vec<User> = Vec::new();
    let mut verified: HashSet<u64> = HashSet::new();
    let mut next_cursor: Option<String> = None;

    let mut omni_url = format!(
        "https://apis.roblox.com/search-api/omni-search?searchQuery={}&sessionId={}&verticalType=user",
        enc(q),
        enc(sid)
    );
    if let Some(tok) = cursor.filter(|s| !s.is_empty()) {
        omni_url.push_str("&pageToken=");
        omni_url.push_str(&enc(tok));
    }

    if let Ok(v) = auth_get_json(&omni_url, cookie) {
        for group in v["searchResults"].as_array().into_iter().flatten() {
            if group["contentGroupType"].as_str() != Some("User") {
                continue;
            }
            for item in group["contents"].as_array().into_iter().flatten() {
                let Some(id) = item["contentId"].as_u64().or_else(|| item["id"].as_u64()) else {
                    continue;
                };
                let name = str_of(&item["username"]);
                let disp = str_of(&item["displayName"]);
                let is_v = item["hasVerifiedBadge"].as_bool() == Some(true);
                if is_v {
                    verified.insert(id);
                }
                if !users.iter().any(|existing| existing.id == id) {
                    users.push(User {
                        id,
                        name: name.clone(),
                        display: if disp.is_empty() { name } else { disp },
                        headshot: String::new(),
                        presence: 0,
                        location: String::new(),
                        place_id: None,
                        verified: is_v,
                    });
                }
            }
        }
        next_cursor = v["nextPageToken"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from);
    }

    if users.is_empty() && cursor.is_none() {
        let v1_url = format!(
            "https://users.roblox.com/v1/users/search?keyword={}&limit=25",
            enc(q)
        );
        let mut req = agent()
            .get(&v1_url)
            .set("Referer", "https://www.roblox.com/");
        if let Some(c) = cookie {
            req = req.set("Cookie", &format!(".ROBLOSECURITY={c}"));
        }
        let v: Value = req
            .call()
            .map_err(|e| e.to_string())?
            .into_json()
            .map_err(|e| e.to_string())?;
        for u in v["data"].as_array().into_iter().flatten() {
            let Some(id) = u["id"].as_u64() else { continue };
            let name = str_of(&u["name"]);
            let disp = str_of(&u["displayName"]);
            let is_v = u["hasVerifiedBadge"].as_bool() == Some(true);
            if is_v {
                verified.insert(id);
            }
            if !users.iter().any(|existing| existing.id == id) {
                users.push(User {
                    id,
                    name: name.clone(),
                    display: if disp.is_empty() { name } else { disp },
                    headshot: String::new(),
                    presence: 0,
                    location: String::new(),
                    place_id: None,
                    verified: is_v,
                });
            }
        }
    }

    enrich_users(&mut users, cookie);
    Ok(UserSearchPage {
        users,
        verified,
        next_cursor,
    })
}

pub fn send_friend_request_api(user_id: u64, cookie: &str) -> Result<(), String> {
    post_csrf(
        &format!("https://friends.roblox.com/v1/users/{user_id}/request-friendship"),
        serde_json::json!({ "friendshipOriginSourceType": 0 }),
        Some(cookie),
        Some("https://www.roblox.com/"),
    )?;
    Ok(())
}

pub fn build_profile(mut user: User) -> Profile {
    enrich_users(std::slice::from_mut(&mut user), None);
    let friends = get_json(&format!(
        "https://friends.roblox.com/v1/users/{}/friends/count",
        user.id
    ))
    .ok()
    .and_then(|v| v["count"].as_u64())
    .unwrap_or(0);
    let avatar = get_json(&format!(
        "https://thumbnails.roblox.com/v1/users/avatar?userIds={}&size=720x720&format=Png&isCircular=false",
        user.id
    ))
    .ok()
    .map(|v| str_of(&v["data"][0]["imageUrl"]))
    .unwrap_or_default();
    Profile {
        user,
        friends,
        avatar,
        robux: None,
    }
}

pub fn fetch_user_currently_wearing(user_id: u64, cookie: Option<&str>) -> Vec<AvatarAsset> {
    let Ok(v) = get_json(&format!("https://avatar.roblox.com/v1/users/{user_id}/avatar")) else {
        return Vec::new();
    };

    let mut items: Vec<AvatarAsset> = Vec::new();
    let mut seen_ids = HashSet::new();

    if let Some(assets) = v["assets"].as_array() {
        for a in assets {
            let Some(id) = a["id"].as_u64() else { continue };
            if !seen_ids.insert(id) { continue; }
            let mut name = str_of(&a["name"]);
            let type_name = str_of(&a["assetType"]["name"]);
            let type_id = a["assetType"]["id"].as_u64().unwrap_or(0);

            if type_name.ends_with("Animation") {
                if let Some(base) = name.split(" - ").next() {
                    name = base.to_string();
                }
            }

            items.push(AvatarAsset {
                id,
                name,
                asset_type_id: type_id,
                asset_type_name: type_name,
                thumb: String::new(),
                meta: None,
                creator_name: String::new(),
                creator_verified: false,
                price: None,
            });
        }
    }

    if let Some(emotes) = v["emotes"].as_array() {
        for e in emotes {
            let Some(id) = e["assetId"].as_u64() else { continue };
            if !seen_ids.insert(id) { continue; }
            let name = str_of(&e["assetName"]);
            items.push(AvatarAsset {
                id,
                name,
                asset_type_id: 61,
                asset_type_name: "Emote".to_string(),
                thumb: String::new(),
                meta: None,
                creator_name: String::new(),
                creator_verified: false,
                price: None,
            });
        }
    }

    let mut deduped: Vec<AvatarAsset> = Vec::new();
    let mut seen_names = HashSet::new();
    for item in items {
        if item.asset_type_name.ends_with("Animation") {
            if !seen_names.insert(item.name.clone()) {
                continue;
            }
        }
        deduped.push(item);
    }

    let ids: Vec<u64> = deduped.iter().map(|a| a.id).collect();
    let thumbs = fetch_asset_thumbs(&ids);
    for a in &mut deduped {
        a.thumb = thumbs.get(&a.id).cloned().unwrap_or_default();
    }

    enrich_assets(&mut deduped, cookie);

    deduped
}

pub fn update_user_description(cookie: &str, description: &str) -> Result<String, String> {
    let body = serde_json::json!({ "description": description });
    let resp = post_csrf(
        "https://users.roblox.com/v1/users/description",
        body,
        Some(cookie),
        Some("https://www.roblox.com/"),
    )?;
    if resp.status() == 200 {
        Ok(description.to_string())
    } else {
        Err(format!("HTTP error {}", resp.status()))
    }
}

pub fn fetch_friend_profile(mut user: User, cookie: Option<String>) -> Result<FriendProfile, String> {
    let id = user.id;
    let info = get_json(&format!("https://users.roblox.com/v1/users/{id}"))?;
    if let Some(n) = info["name"].as_str() {
        user.name = n.to_string();
    }
    if let Some(d) = info["displayName"].as_str() {
        user.display = d.to_string();
    }
    let description = str_of(&info["description"]);
    let created = str_of(&info["created"]).chars().take(10).collect();
    let verified = info["hasVerifiedBadge"].as_bool().unwrap_or(false);
    user.verified = verified;

    enrich_users(std::slice::from_mut(&mut user), cookie.as_deref());

    let count_of = |endpoint: &str| -> u64 {
        get_json(&format!("https://friends.roblox.com/v1/users/{id}/{endpoint}/count"))
            .ok()
            .and_then(|v| v["count"].as_u64())
            .unwrap_or(0)
    };
    let friends = count_of("friends");
    let followers = count_of("followers");
    let following = count_of("followings");

    let avatar = get_json(&format!(
        "https://thumbnails.roblox.com/v1/users/avatar?userIds={id}&size=720x720&format=Png&isCircular=false"
    ))
    .ok()
    .map(|v| str_of(&v["data"][0]["imageUrl"]))
    .unwrap_or_default();

    let playing_game = user
        .place_id
        .and_then(|pid| games_by_places(&[pid]).into_iter().next());

    let exp_ids: Vec<u64> = get_json(&format!(
        "https://games.roblox.com/v2/users/{id}/games?accessFilter=2&limit=12"
    ))
    .ok()
    .map(|v| {
        v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|g| g["id"].as_u64())
            .collect()
    })
    .unwrap_or_default();
    let experiences = if exp_ids.is_empty() {
        Vec::new()
    } else {
        games_by_universe(&exp_ids)
    };

    let currently_wearing = fetch_user_currently_wearing(id, cookie.as_deref());

    Ok(FriendProfile {
        user,
        description,
        created,
        verified,
        friends,
        followers,
        following,
        avatar,
        playing_game,
        experiences,
        currently_wearing,
    })
}

pub fn fetch_robux_balance(user_id: u64, cookie: &str) -> Option<u64> {
    let urls = [
        format!("https://economy.roblox.com/v1/users/{user_id}/currency"),
        "https://economy.roblox.com/v1/user/currency".to_string(),
    ];
    for url in &urls {
        if let Ok(v) = auth_get_json(url, Some(cookie)) {
            if let Some(robux) = v["robux"].as_u64() {
                return Some(robux);
            }
        }
    }
    None
}

pub fn fetch_authenticated_session(cookie: &str) -> Result<(Profile, Vec<User>), String> {
    let me_json = auth_get_json("https://users.roblox.com/v1/users/authenticated", Some(cookie))?;
    let id = me_json["id"]
        .as_u64()
        .ok_or_else(|| "Session expired or invalid".to_string())?;
    let user = User {
        id,
        name: str_of(&me_json["name"]),
        display: str_of(&me_json["displayName"]),
        headshot: String::new(),
        presence: 1,
        location: String::new(),
        place_id: None,
        verified: me_json["hasVerifiedBadge"].as_bool().unwrap_or(false),
    };
    let mut profile = build_profile(user);
    profile.robux = fetch_robux_balance(id, cookie);

    let mut friends = match auth_get_json(
        &format!("https://friends.roblox.com/v1/users/{id}/friends"),
        Some(cookie),
    ) {
        Ok(v) => {
            let mut list: Vec<User> = v["data"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|u| {
                    Some(User {
                        id: u["id"].as_u64()?,
                        name: str_of(&u["name"]),
                        display: str_of(&u["displayName"]),
                        headshot: String::new(),
                        presence: 0,
                        location: String::new(),
                        place_id: None,
                        verified: u["hasVerifiedBadge"].as_bool().unwrap_or(false),
                    })
                })
                .collect();
            enrich_users(&mut list, Some(cookie));
            list
        }
        Err(_) => Vec::new(),
    };

    if friends.is_empty() {
        if let Ok(v) = auth_get_json(
            &format!("https://friends.roblox.com/v1/users/{id}/friends/find?limit=50"),
            Some(cookie),
        ) {
            let ids: Vec<u64> = v["PageItems"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|x| x["id"].as_u64())
                .collect();
            if !ids.is_empty() {
                if let Ok(list) = fetch_users_by_ids(&ids, Some(cookie)) {
                    friends = list;
                }
            }
        }
    }

    friends.sort_by_key(|f| match f.presence {
        2 => 0,
        1 => 1,
        3 => 2,
        _ => 3,
    });

    Ok((profile, friends))
}

pub fn parse_hex_color(hex: &str) -> Color32 {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            return Color32::from_rgb(r, g, b);
        }
    }
    Color32::from_rgb(163, 162, 165)
}

pub fn asset_type_id_from_name(name: &str) -> u64 {
    match name {
        "TShirt" | "T-Shirt" => 2,
        "Hat" => 8,
        "Shirt" => 11,
        "Pants" => 12,
        "Head" => 17,
        "Face" => 18,
        "Gear" => 19,
        "Torso" => 27,
        "RightArm" | "Right Arm" => 28,
        "LeftArm" | "Left Arm" => 29,
        "LeftLeg" | "Left Leg" => 30,
        "RightLeg" | "Right Leg" => 31,
        "HairAccessory" | "Hair Accessory" => 41,
        "FaceAccessory" | "Face Accessory" => 42,
        "NeckAccessory" | "Neck Accessory" => 43,
        "ShoulderAccessory" | "Shoulder Accessory" => 44,
        "FrontAccessory" | "Front Accessory" => 45,
        "BackAccessory" | "Back Accessory" => 46,
        "WaistAccessory" | "Waist Accessory" => 47,
        "ClimbAnimation" => 48,
        "DeathAnimation" => 49,
        "FallAnimation" => 50,
        "IdleAnimation" => 51,
        "JumpAnimation" => 52,
        "RunAnimation" => 53,
        "SwimAnimation" => 54,
        "WalkAnimation" => 55,
        "PoseAnimation" => 56,
        "EmoteAnimation" => 61,
        "TShirtAccessory" => 64,
        "ShirtAccessory" => 65,
        "PantsAccessory" => 66,
        "JacketAccessory" => 67,
        "SweaterAccessory" => 68,
        "ShortsAccessory" => 69,
        "LeftShoeAccessory" => 70,
        "RightShoeAccessory" => 71,
        "DressSkirtAccessory" => 72,
        "EyebrowAccessory" => 76,
        "EyelashAccessory" => 77,
        "MoodAnimation" => 78,
        "DynamicHead" => 79,
        "FaceMakeup" => 88,
        "LipMakeup" => 89,
        "EyeMakeup" => 90,
        _ => 0,
    }
}

pub fn fetch_asset_thumbs(ids: &[u64]) -> HashMap<u64, String> {
    let mut map = HashMap::new();
    for chunk in ids.chunks(50) {
        if chunk.is_empty() {
            continue;
        }
        if let Ok(v) = get_json(&format!(
            "https://thumbnails.roblox.com/v1/assets?assetIds={}&size=150x150&format=Png&isCircular=false",
            join(chunk)
        )) {
            for d in v["data"].as_array().into_iter().flatten() {
                if let (Some(id), Some(url)) = (d["targetId"].as_u64(), d["imageUrl"].as_str()) {
                    if !url.is_empty() {
                        map.insert(id, url.to_string());
                    }
                }
            }
        }
    }
    map
}

pub fn parse_avatar_state_json(v: &Value) -> AvatarState {
    let avatar_type = match v["playerAvatarType"].as_str() {
        Some("R6") => "R6".to_string(),
        _ => "R15".to_string(),
    };
    let s = &v["scales"];
    let scales = AvatarScales {
        height: s["height"].as_f64().unwrap_or(1.0) as f32,
        width: s["width"].as_f64().unwrap_or(1.0) as f32,
        head: s["head"].as_f64().unwrap_or(1.0) as f32,
        depth: s["depth"].as_f64().unwrap_or(1.0) as f32,
        proportion: s["proportion"].as_f64().unwrap_or(0.0) as f32,
        body_type: s["bodyType"].as_f64().unwrap_or(0.0) as f32,
    };
    let c = &v["bodyColors"];
    let colors = BodyColors {
        head: c["headColorId"].as_u64().unwrap_or(194),
        torso: c["torsoColorId"].as_u64().unwrap_or(23),
        right_arm: c["rightArmColorId"].as_u64().unwrap_or(194),
        left_arm: c["leftArmColorId"].as_u64().unwrap_or(194),
        right_leg: c["rightLegColorId"].as_u64().unwrap_or(102),
        left_leg: c["leftLegColorId"].as_u64().unwrap_or(102),
    };
    let mut wearing: Vec<AvatarAsset> = v["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let id = a["id"].as_u64()?;
            let name = str_of(&a["name"]);
            let type_name = str_of(&a["assetType"]["name"]);
            let type_id = a["assetType"]["id"]
                .as_u64()
                .unwrap_or_else(|| asset_type_id_from_name(&type_name));
            let meta = if a["meta"].is_null() {
                None
            } else {
                Some(a["meta"].clone())
            };
            Some(AvatarAsset {
                id,
                name,
                asset_type_id: type_id,
                asset_type_name: type_name,
                thumb: String::new(),
                meta,
                creator_name: String::new(),
                creator_verified: false,
                price: None,
            })
        })
        .collect();
    let ids: Vec<u64> = wearing.iter().map(|a| a.id).collect();
    let thumbs = fetch_asset_thumbs(&ids);
    for a in &mut wearing {
        a.thumb = thumbs.get(&a.id).cloned().unwrap_or_default();
    }
    AvatarState {
        avatar_type,
        scales,
        colors,
        wearing,
    }
}

pub fn enrich_assets(assets: &mut [AvatarAsset], cookie: Option<&str>) {
    if assets.is_empty() {
        return;
    }
    for chunk in assets.chunks_mut(30) {
        let items: Vec<Value> = chunk
            .iter()
            .map(|a| serde_json::json!({ "itemType": 1, "id": a.id }))
            .collect();
        if let Ok(resp) = post_csrf(
            "https://catalog.roblox.com/v1/catalog/items/details",
            serde_json::json!({ "items": items }),
            cookie,
            Some("https://www.roblox.com/"),
        ) {
            if let Ok(v) = resp.into_json::<Value>() {
                for item in v["data"].as_array().into_iter().flatten() {
                    let Some(id) = item["id"].as_u64() else { continue };
                    if let Some(a) = chunk.iter_mut().find(|a| a.id == id) {
                        let cname = str_of(&item["creatorName"]);
                        if !cname.is_empty() {
                            a.creator_name = cname;
                        }
                        a.creator_verified = item["creatorHasVerifiedBadge"]
                            .as_bool()
                            .unwrap_or(false);
                        a.price = item["price"].as_u64().or_else(|| item["lowestPrice"].as_u64());
                        if let Some(n) = item["name"].as_str() {
                            if !n.is_empty() && !a.asset_type_name.ends_with("Animation") {
                                a.name = n.to_string();
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn fetch_avatar_state(cookie: &str) -> Result<AvatarState, String> {
    let v = auth_get_json("https://avatar.roblox.com/v1/avatar", Some(cookie))?;
    let mut state = parse_avatar_state_json(&v);
    enrich_assets(&mut state.wearing, Some(cookie));
    Ok(state)
}

pub fn fetch_avatar_palette() -> Vec<PaletteColor> {
    let Ok(v) = get_json("https://avatar.roblox.com/v1/avatar-rules") else {
        return Vec::new();
    };
    v["bodyColorsPalette"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            Some(PaletteColor {
                id: c["brickColorId"].as_u64()?,
                name: str_of(&c["name"]),
                color: parse_hex_color(&str_of(&c["hexColor"])),
            })
        })
        .collect()
}

pub fn fetch_avatar_session(cookie: &str) -> Result<(AvatarState, Vec<PaletteColor>), String> {
    let state = fetch_avatar_state(cookie)?;
    let palette = fetch_avatar_palette();
    Ok((state, palette))
}

pub fn fetch_avatar_inventory(
    user_id: u64,
    tab: AvatarTab,
    cookie: &str,
    cursor: Option<&str>,
) -> Result<(Vec<AvatarAsset>, Option<String>), String> {
    let Some(types) = tab.inventory_query() else {
        return Ok((Vec::new(), None));
    };
    let mut url = format!(
        "https://inventory.roblox.com/v2/users/{user_id}/inventory?assetTypes={types}&limit=50&sortOrder=Desc"
    );
    if let Some(c) = cursor.filter(|s| !s.is_empty()) {
        url.push_str("&cursor=");
        url.push_str(&enc(c));
    }
    let v = auth_get_json(&url, Some(cookie))?;
    let next_cursor = v["nextPageCursor"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let mut items: Vec<AvatarAsset> = v["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|it| {
            let id = it["assetId"].as_u64()?;
            let name = str_of(&it["name"]);
            let type_name = str_of(&it["assetType"]);
            let type_id = asset_type_id_from_name(&type_name);
            Some(AvatarAsset {
                id,
                name,
                asset_type_id: type_id,
                asset_type_name: type_name,
                thumb: String::new(),
                meta: None,
                creator_name: String::new(),
                creator_verified: false,
                price: None,
            })
        })
        .collect();
    let ids: Vec<u64> = items.iter().map(|a| a.id).collect();
    let thumbs = fetch_asset_thumbs(&ids);
    for a in &mut items {
        a.thumb = thumbs.get(&a.id).cloned().unwrap_or_default();
    }
    enrich_assets(&mut items, Some(cookie));
    Ok((items, next_cursor))
}

pub fn fetch_avatar_outfits(
    user_id: u64,
    page: u32,
    cookie: &str,
) -> Result<(Vec<OutfitItem>, bool), String> {
    let v = auth_get_json(
        &format!(
            "https://avatar.roblox.com/v1/users/{user_id}/outfits?page={page}&itemsPerPage=50"
        ),
        Some(cookie),
    )?;
    let mut outfits: Vec<OutfitItem> = v["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|o| {
            Some(OutfitItem {
                id: o["id"].as_u64()?,
                name: str_of(&o["name"]),
                thumb: String::new(),
            })
        })
        .collect();
    let has_more = outfits.len() >= 50;
    let ids: Vec<u64> = outfits.iter().map(|o| o.id).collect();
    for chunk in ids.chunks(50) {
        if chunk.is_empty() {
            continue;
        }
        if let Ok(t) = get_json(&format!(
            "https://thumbnails.roblox.com/v1/users/outfits?userOutfitIds={}&size=150x150&format=Png&isCircular=false",
            join(chunk)
        )) {
            let map: HashMap<u64, String> = t["data"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|d| Some((d["targetId"].as_u64()?, str_of(&d["imageUrl"]))))
                .collect();
            for o in &mut outfits {
                if let Some(url) = map.get(&o.id) {
                    o.thumb = url.clone();
                }
            }
        }
    }
    Ok((outfits, has_more))
}

pub fn fetch_user_renders(user_id: u64) -> (String, String) {
    let mut avatar_url = String::new();
    let mut headshot_url = String::new();
    for attempt in 0..5 {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(450));
        }
        if avatar_url.is_empty() {
            if let Ok(v) = get_json(&format!(
                "https://thumbnails.roblox.com/v1/users/avatar?userIds={user_id}&size=720x720&format=Png&isCircular=false"
            )) {
                let state = v["data"][0]["state"].as_str().unwrap_or("");
                let url = str_of(&v["data"][0]["imageUrl"]);
                if state == "Completed" && !url.is_empty() {
                    avatar_url = url;
                }
            }
        }
        if headshot_url.is_empty() {
            if let Ok(v) = get_json(&format!(
                "https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds={user_id}&size=150x150&format=Png&isCircular=false"
            )) {
                let state = v["data"][0]["state"].as_str().unwrap_or("");
                let url = str_of(&v["data"][0]["imageUrl"]);
                if state == "Completed" && !url.is_empty() {
                    headshot_url = url;
                }
            }
        }
        if !avatar_url.is_empty() && !headshot_url.is_empty() {
            break;
        }
    }
    (avatar_url, headshot_url)
}

pub fn avatar_assets_payload(assets: &[AvatarAsset]) -> Value {
    let arr: Vec<Value> = assets
        .iter()
        .map(|a| {
            let mut obj = serde_json::json!({ "id": a.id });
            if let Some(m) = &a.meta {
                obj["meta"] = m.clone();
            }
            obj
        })
        .collect();
    serde_json::json!({ "assets": arr })
}

pub fn is_single_slot_asset(asset_type_id: u64) -> bool {
    matches!(
        asset_type_id,
        2 | 11 | 12 | 17 | 18 | 19 | 27 | 28 | 29 | 30 | 31 | 48 | 49 | 50 | 51 | 52 | 53 | 54 | 55
            | 56 | 78 | 79
    )
}

pub fn compute_toggled_assets(wearing: &[AvatarAsset], item: &AvatarAsset) -> (Vec<AvatarAsset>, bool) {
    if wearing.iter().any(|w| w.id == item.id) {
        let next = wearing
            .iter()
            .filter(|w| w.id != item.id)
            .cloned()
            .collect();
        return (next, false);
    }
    let single = if item.asset_type_id != 0 {
        is_single_slot_asset(item.asset_type_id)
    } else {
        is_single_slot_asset(asset_type_id_from_name(&item.asset_type_name))
    };
    let mut next: Vec<AvatarAsset> = if single {
        wearing
            .iter()
            .filter(|w| {
                if item.asset_type_id != 0 && w.asset_type_id != 0 {
                    w.asset_type_id != item.asset_type_id
                } else {
                    !w.asset_type_name.eq_ignore_ascii_case(&item.asset_type_name)
                }
            })
            .cloned()
            .collect()
    } else {
        wearing.to_vec()
    };
    next.push(item.clone());
    (next, true)
}

pub fn post_wearing_assets(cookie: &str, assets: &[AvatarAsset]) -> Result<(), String> {
    let resp = post_csrf(
        "https://avatar.roblox.com/v2/avatar/set-wearing-assets",
        avatar_assets_payload(assets),
        Some(cookie),
        Some("https://www.roblox.com/my/avatar"),
    )?;
    if let Ok(v) = resp.into_json::<Value>() {
        if v["success"].as_bool() == Some(false) {
            return Err("Roblox could not equip one or more selected items.".to_string());
        }
    }
    Ok(())
}

pub fn finish_avatar_update(
    user_id: u64,
    cookie: &str,
    note: String,
) -> Result<(AvatarState, String, String, String), String> {
    let state = fetch_avatar_state(cookie)?;
    let (avatar_url, headshot_url) = fetch_user_renders(user_id);
    Ok((state, avatar_url, headshot_url, note))
}

pub fn apply_avatar_assets(
    user_id: u64,
    cookie: &str,
    assets: &[AvatarAsset],
    note: String,
) -> Result<(AvatarState, String, String, String), String> {
    post_wearing_assets(cookie, assets)?;
    finish_avatar_update(user_id, cookie, note)
}

pub fn apply_avatar_type(
    user_id: u64,
    cookie: &str,
    avatar_type: &str,
) -> Result<(AvatarState, String, String, String), String> {
    post_csrf(
        "https://avatar.roblox.com/v1/avatar/set-player-avatar-type",
        serde_json::json!({ "playerAvatarType": avatar_type }),
        Some(cookie),
        Some("https://www.roblox.com/my/avatar"),
    )?;
    finish_avatar_update(user_id, cookie, format!("Switched avatar rig to {avatar_type}."))
}

pub fn apply_body_colors(
    user_id: u64,
    cookie: &str,
    colors: BodyColors,
) -> Result<(AvatarState, String, String, String), String> {
    post_csrf(
        "https://avatar.roblox.com/v1/avatar/set-body-colors",
        serde_json::json!({
            "headColorId": colors.head,
            "torsoColorId": colors.torso,
            "rightArmColorId": colors.right_arm,
            "leftArmColorId": colors.left_arm,
            "rightLegColorId": colors.right_leg,
            "leftLegColorId": colors.left_leg,
        }),
        Some(cookie),
        Some("https://www.roblox.com/my/avatar"),
    )?;
    finish_avatar_update(user_id, cookie, "Updated avatar skin tone.".to_string())
}

pub fn apply_avatar_scales(
    user_id: u64,
    cookie: &str,
    scales: AvatarScales,
) -> Result<(AvatarState, String, String, String), String> {
    post_csrf(
        "https://avatar.roblox.com/v1/avatar/set-scales",
        serde_json::json!({
            "height": (scales.height * 100.0).round() / 100.0,
            "width": (scales.width * 100.0).round() / 100.0,
            "head": (scales.head * 100.0).round() / 100.0,
            "depth": (scales.depth * 100.0).round() / 100.0,
            "proportion": (scales.proportion * 100.0).round() / 100.0,
            "bodyType": (scales.body_type * 100.0).round() / 100.0,
        }),
        Some(cookie),
        Some("https://www.roblox.com/my/avatar"),
    )?;
    finish_avatar_update(user_id, cookie, "Updated avatar body scales.".to_string())
}

pub fn apply_wear_outfit(
    user_id: u64,
    cookie: &str,
    outfit: &OutfitItem,
) -> Result<(AvatarState, String, String, String), String> {
    let details = auth_get_json(
        &format!("https://avatar.roblox.com/v1/outfits/{}/details", outfit.id),
        Some(cookie),
    )?;
    let parsed = parse_avatar_state_json(&details);
    let _ = post_csrf(
        "https://avatar.roblox.com/v1/avatar/set-player-avatar-type",
        serde_json::json!({ "playerAvatarType": parsed.avatar_type }),
        Some(cookie),
        Some("https://www.roblox.com/my/avatar"),
    );
    let _ = post_csrf(
        "https://avatar.roblox.com/v1/avatar/set-body-colors",
        serde_json::json!({
            "headColorId": parsed.colors.head,
            "torsoColorId": parsed.colors.torso,
            "rightArmColorId": parsed.colors.right_arm,
            "leftArmColorId": parsed.colors.left_arm,
            "rightLegColorId": parsed.colors.right_leg,
            "leftLegColorId": parsed.colors.left_leg,
        }),
        Some(cookie),
        Some("https://www.roblox.com/my/avatar"),
    );
    let s = &details["scale"];
    if s.is_object() {
        let _ = post_csrf(
            "https://avatar.roblox.com/v1/avatar/set-scales",
            s.clone(),
            Some(cookie),
            Some("https://www.roblox.com/my/avatar"),
        );
    }
    post_wearing_assets(cookie, &parsed.wearing)?;
    finish_avatar_update(user_id, cookie, format!("Equipped outfit \"{}\".", outfit.name))
}

pub fn start_quick_login(tx: Sender<Msg>, ctx: egui::Context) -> Arc<AtomicBool> {
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    thread::spawn(move || {
        let send = |m: Msg| {
            let _ = tx.send(m);
            ctx.request_repaint();
        };

        let created = match post_json(
            "https://apis.roblox.com/auth-token-service/v1/login/create",
            serde_json::json!({}),
        ) {
            Ok(v) => v,
            Err(e) => {
                send(Msg::QuickLoginDone(Err(format!(
                    "Could not start Quick Login: {e}"
                ))));
                return;
            }
        };

        let Some(code) = created["code"].as_str().map(str::to_string) else {
            send(Msg::QuickLoginDone(Err(
                "Unexpected response from Roblox Quick Login".to_string(),
            )));
            return;
        };
        let Some(private_key) = created["privateKey"].as_str().map(str::to_string) else {
            send(Msg::QuickLoginDone(Err(
                "Missing private key from Roblox Quick Login".to_string(),
            )));
            return;
        };
        let image_path = str_of(&created["imagePath"]);
        let qr_bytes = if !image_path.is_empty() {
            let qr_url = format!("https://apis.roblox.com/auth-token-service{image_path}");
            agent()
                .get(&qr_url)
                .call()
                .ok()
                .and_then(|resp| {
                    let mut buf = Vec::new();
                    resp.into_reader().take(64 * 1024).read_to_end(&mut buf).ok()?;
                    Some(buf)
                })
        } else {
            None
        };

        send(Msg::QuickLoginCreated {
            code: code.clone(),
            qr_bytes,
        });

        while !flag.load(Ordering::Relaxed) {
            for _ in 0..15 {
                if flag.load(Ordering::Relaxed) {
                    return;
                }
                thread::sleep(Duration::from_millis(200));
            }
            if flag.load(Ordering::Relaxed) {
                return;
            }

            let resp = match post_csrf(
                "https://apis.roblox.com/auth-token-service/v1/login/status",
                serde_json::json!({ "code": code, "privateKey": private_key }),
                None,
                None,
            ) {
                Ok(r) => r,
                Err(e) => {
                    if e.contains("400") || e.contains("404") {
                        send(Msg::QuickLoginDone(Err(
                            "Quick Login code expired. Click New code to try again.".to_string(),
                        )));
                        return;
                    }
                    continue;
                }
            };

            let Ok(v) = resp.into_json::<Value>() else {
                continue;
            };
            let status = str_of(&v["status"]);
            let account = v["accountName"].as_str().map(str::to_string);

            if status == "Validated" {
                match post_csrf(
                    "https://auth.roblox.com/v2/login",
                    serde_json::json!({
                        "ctype": "AuthToken",
                        "cvalue": code,
                        "password": private_key
                    }),
                    None,
                    None,
                ) {
                    Ok(login_resp) => {
                        if let Some(cookie) = extract_roblosecurity(&login_resp) {
                            send(Msg::QuickLoginDone(Ok(cookie)));
                        } else {
                            send(Msg::QuickLoginDone(Err(
                                "Login approved, but Roblox did not return a session cookie."
                                    .to_string(),
                            )));
                        }
                    }
                    Err(e) => {
                        send(Msg::QuickLoginDone(Err(format!("Login failed: {e}"))));
                    }
                }
                return;
            } else {
                send(Msg::QuickLoginStatus {
                    code: code.clone(),
                    status,
                    account,
                });
            }
        }
    });
    cancel
}
