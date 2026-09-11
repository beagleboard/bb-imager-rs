use std::path::Path;

use tokio::task::JoinSet;

mod armbian;
mod fedora;
mod helpers;

#[tokio::main]
async fn main() {
    let dist = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("dist")
        .join("os_list.json");

    let downloader = reqwest::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .unwrap();

    let mut tasks = JoinSet::new();

    tasks.spawn(armbian::os_list_items(downloader.clone()));
    tasks.spawn(fedora::os_list_items(downloader));

    let res = tasks.join_all().await;

    let os_list = bb_config::Config {
        os_list: res.into_iter().flatten().collect(),
        ..Default::default()
    };

    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(dist.parent().unwrap()).unwrap();
        let dest = std::fs::File::create(dist).unwrap();
        serde_json::to_writer(dest, &os_list).unwrap();
    })
    .await
    .unwrap();
}
