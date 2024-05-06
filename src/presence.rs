use discord_sdk as ds;
use ds::activity::{self, Activity, ActivityArgs, Assets, Button, ButtonKind, IntoTimestamp, Timestamps};
use tokio::sync::MutexGuard;

pub const APP_ID: ds::AppId = 1236161402050183238;

pub struct Client {
    pub discord: ds::Discord,
    pub user: ds::user::User,
    pub wheel: ds::wheel::Wheel,
}

pub async fn make_client(subs: ds::Subscriptions) -> Client {
    let (wheel, handler) = ds::wheel::Wheel::new(Box::new(|err| {
        eprintln!("[DISCORD] Encountered an error: {err}");
    }));

    let mut user = wheel.user();

    let discord = ds::Discord::new(ds::DiscordApp::PlainId(APP_ID), subs, Box::new(handler))
        .expect("[DISCORD] Unable to create discord client");

    println!("[DISCORD] Waiting for handshake...");
    user.0.changed().await.unwrap();

    let user = match &*user.0.borrow() {
        ds::wheel::UserState::Connected(user) => user.clone(),
        ds::wheel::UserState::Disconnected(err) => panic!("failed to connect to Discord: {}", err),
    };

    println!(
        "[DISCORD] Connected to Discord, local user is {}",
        user.username
    );

    Client {
        discord,
        user,
        wheel,
    }
}

pub async fn update_presence(
    client: MutexGuard<'_, Client>,
    song: &crate::shazam::core::thread_messages::SongRecognizedMessage,
) {
    let button: ButtonKind = ButtonKind::Link(Button {
        label: "View GitHub".to_string(),
        url: "https://github.com/barnabwhy/song_id".to_string(),
    });

    let mut activity = Activity {
        state: Some(song.artist_name.to_string()),
        details: Some(song.song_name.to_string()),
        assets: None,
        timestamps: None,
        party: None,
        buttons_or_secrets: Some(activity::ButtonsOrSecrets::Buttons {
            buttons: vec![button],
        }),
        kind: ds::activity::ActivityKind::Listening,
        instance: false,
    };

    if let Some(cover_image) = &song.cover_image {
        activity.assets = Some(Assets::default().large(cover_image, song.album_name.to_owned()));
    }

    if let Some(seek) = song.track_seek {
        let timestamp = song.timestamp.into_timestamp()
            - seek as i64
            - (song.signature.number_samples as i64 / song.signature.sample_rate_hz as i64);
        activity.timestamps = Some(Timestamps {
            start: Some(timestamp),
            end: None,
        });
    }

    let mut activity_args = ActivityArgs::default();
    activity_args.activity = Some(activity);

    client
        .discord
        .update_activity(activity_args)
        .await
        .expect("[DISCORD] Failed to update presence");

    println!("[DISCORD] Updated presence");
}
