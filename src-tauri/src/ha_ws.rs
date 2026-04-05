use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::state::Config;

#[derive(Deserialize, Debug)]
struct HaMessage {
    #[serde(rename = "type")]
    msg_type: String,
    id: Option<u64>,
    #[serde(flatten)]
    rest: serde_json::Value,
}

// Subscription IDs — lets us route incoming events to the right handler.
struct Subscriptions {
    persistent_notification: u64,
    notify_service_call: u64,
}

#[derive(Deserialize, Debug)]
struct PnNotification {
    title: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize, Debug)]
struct PnEvent {
    notifications: Option<std::collections::HashMap<String, PnNotification>>,
}

pub async fn start(app: AppHandle, url: String, token: String) {
    let ws_url = url
        .replacen("https://", "wss://", 1)
        .replacen("http://", "ws://", 1);
    let ws_url = format!("{}/api/websocket", ws_url.trim_end_matches('/'));

    loop {
        if let Err(e) = run(&app, &ws_url, &token).await {
            #[cfg(debug_assertions)]
            eprintln!("[ha_ws] disconnected: {e}, reconnecting in 10s");
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        if !is_configured(&app) {
            break;
        }
    }
}

fn is_configured(app: &AppHandle) -> bool {
    let config = app.state::<Mutex<Config>>();
    let config = config.lock().unwrap();
    config.url.is_some() && config.token.is_some()
}

fn notifications_enabled(app: &AppHandle) -> bool {
    let config = app.state::<Mutex<Config>>();
    let config = config.lock().unwrap();
    config.notifications_enabled
}

async fn run(
    app: &AppHandle,
    ws_url: &str,
    token: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (ws_stream, _) = connect_async(ws_url).await?;
    let (mut write, mut read) = ws_stream.split();

    let mut msg_id: u64 = 1;
    let mut subs: Option<Subscriptions> = None;

    while let Some(msg) = read.next().await {
        let msg = msg?;
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => return Ok(()),
            _ => continue,
        };

        let ha_msg: HaMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(_) => continue,
        };

        match ha_msg.msg_type.as_str() {
            "auth_required" => {
                write
                    .send(Message::text(
                        serde_json::json!({"type": "auth", "access_token": token}).to_string(),
                    ))
                    .await?;
            }
            "auth_ok" => {
                // 1. Persistent notifications (shown in the HA UI bell icon)
                let pn_id = msg_id;
                write
                    .send(Message::text(
                        serde_json::json!({
                            "id": msg_id,
                            "type": "persistent_notification/subscribe"
                        })
                        .to_string(),
                    ))
                    .await?;
                msg_id += 1;

                // 2. call_service events filtered to the notify domain.
                //    Catches notify.notify, notify.mobile_app_*, etc.
                let notify_id = msg_id;
                write
                    .send(Message::text(
                        serde_json::json!({
                            "id": msg_id,
                            "type": "subscribe_trigger",
                            "trigger": {
                                "platform": "event",
                                "event_type": "call_service",
                                "event_data": {
                                    "domain": "notify"
                                }
                            }
                        })
                        .to_string(),
                    ))
                    .await?;
                msg_id += 1;

                subs = Some(Subscriptions {
                    persistent_notification: pn_id,
                    notify_service_call: notify_id,
                });
            }
            "auth_invalid" => {
                return Err("HA authentication failed — check your Long-Lived Access Token".into());
            }
            "event" => {
                let event_id = ha_msg.id;
                if let Some(ref s) = subs {
                    if event_id == Some(s.persistent_notification) {
                        handle_pn_event(app, &ha_msg.rest);
                    } else if event_id == Some(s.notify_service_call) {
                        handle_notify_event(app, &ha_msg.rest);
                    } else {
                        // Fallback: try both handlers (e.g. first event before subs is set)
                        handle_pn_event(app, &ha_msg.rest);
                        handle_notify_event(app, &ha_msg.rest);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

// ── persistent_notification/subscribe handler ─────────────────────────────────

fn handle_pn_event(app: &AppHandle, payload: &serde_json::Value) {
    let event = match payload.get("event") {
        Some(e) => e,
        None => return,
    };

    let pn: PnEvent = match serde_json::from_value(event.clone()) {
        Ok(p) => p,
        Err(_) => return,
    };

    let notifications = match pn.notifications {
        Some(n) => n,
        None => return,
    };

    for notif in notifications.values() {
        let title = notif.title.as_deref().unwrap_or("Home Assistant");
        let body = notif.message.as_deref().unwrap_or("");
        if body.is_empty() {
            continue;
        }
        if notifications_enabled(app) {
            let _ = app.notification().builder().title(title).body(body).show();
        }
    }
}

// ── call_service / notify domain handler ─────────────────────────────────────
//
// HA fires a call_service event for every service call. When an automation
// calls notify.notify (or notify.mobile_app_*, etc.) the event_data looks like:
//
//   { "domain": "notify", "service": "notify",
//     "service_data": { "message": "...", "title": "..." } }

fn handle_notify_event(app: &AppHandle, payload: &serde_json::Value) {
    // subscribe_trigger wraps as: { "variables": { "trigger": { "event": { ... } } } }
    let service_data = payload
        .pointer("/variables/trigger/event/data/service_data")
        .or_else(|| payload.pointer("/event/data/service_data"));

    let service_data = match service_data {
        Some(d) => d,
        None => return,
    };

    let message = service_data
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if message.is_empty() || message == "clear_notification" {
        return;
    }

    if !notifications_enabled(app) {
        return;
    }

    let title = service_data
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Home Assistant");

    let _ = app.notification().builder().title(title).body(message).show();
}


