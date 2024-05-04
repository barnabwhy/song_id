use discord_sdk as ds;
use ds::activity::IntoTimestamp;
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

    println!("[DISCORD] Connected to Discord, local user is {}", user.username);

    Client {
        discord,
        user,
        wheel,
    }
}

pub async fn update_presence(client: MutexGuard<'_, Client>, song: &crate::shazam::core::thread_messages::SongRecognizedMessage) {
    let mut rp = ds::activity::ActivityBuilder::default()
        .details(song.song_name.to_owned())
        .state(song.artist_name.to_owned())
        .assets(
            ds::activity::Assets::default()
                .large(song.cover_image.to_owned().unwrap(), song.album_name.to_owned()),
        )
        .button(
            ds::activity::Button {
                label: "View GitHub".to_owned(),
                url: "https://github.com/barnabwhy/song_id".to_owned(),
            }
        );

    if let Some(seek) = song.track_seek {
        let timestamp = song.timestamp.into_timestamp() - seek as i64;
        rp = rp.start_timestamp(timestamp);
    }
    
    client.discord.update_activity(rp).await
        .expect("[DISCORD] Failed to update presence");

    println!("[DISCORD] Updated presence");
}