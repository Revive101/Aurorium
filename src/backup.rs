use eventsource_client::{Client, ClientBuilder};
use futures_util::{StreamExt, stream};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    fs::{File, create_dir_all},
    io::AsyncWriteExt,
    time::sleep,
};

use crate::{ARGS, models::revision::LocalRevision};

pub struct BackupClient;

impl BackupClient {
    pub async fn new(mirror_host: SocketAddr) {
        let client = ClientBuilder::for_url(&format!("http://{}/mirror", mirror_host)).unwrap().build();

        let mut stream = client.stream();

        while let Some(event) = stream.next().await {
            match event {
                Ok(eventsource_client::SSE::Event(ev)) => {
                    if let Ok(files) = serde_json::from_str(&ev.data) {
                        Self::backup(files, mirror_host).await;
                        LocalRevision::init_all(&ARGS.save_directory).await.unwrap();
                    }
                }
                Ok(eventsource_client::SSE::Connected(_)) => {
                    println!("Connected to host!");
                }
                Ok(eventsource_client::SSE::Comment(_)) => (),
                Err(e) => {
                    eprintln!("Error receiving event: {:?}", e);
                    sleep(Duration::from_secs(10)).await;
                }
            }
        }
    }

    async fn backup(file_list: Vec<String>, mirror_host: SocketAddr) {
        let client = reqwest::Client::builder().user_agent("KingsIsle Patcher").build().unwrap();

        let multi_pb = MultiProgress::new();
        let main_style = ProgressStyle::with_template("{spinner:.blue} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("#>-");

        let main_pb = multi_pb.add(ProgressBar::new(file_list.len() as u64));
        main_pb.set_style(main_style);

        let download_style = ProgressStyle::with_template(
            "{msg:.cyan} {spinner:.blue} [{elapsed_precise}] [{wide_bar:.green/blue}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-");

        let download_futures = file_list.into_iter().map(|file| {
            let client = client.clone();
            // Progress bar
            let multi_pb = multi_pb.clone();
            let main_pb = main_pb.clone();
            let style = download_style.clone();

            async move {
                let url = format!("http://{}/mirror/files/{}", mirror_host, file);
                let path = ARGS.save_directory.join(&file);

                if !path.exists() {
                    let file_pb = multi_pb.add(ProgressBar::new_spinner());
                    file_pb.set_style(style);
                    file_pb.set_message(format!("Downloading {}", file));

                    if let Ok(res) = client.get(&url).send().await {
                        file_pb.set_length(res.content_length().unwrap_or(0));

                        if let Err(e) = Self::write_to_file_chunked_with_progress(&path, res, &file_pb).await {
                            file_pb.finish_with_message(format!("Failed: {e}"));
                        } else {
                            file_pb.finish_with_message("Done");
                        }

                        multi_pb.remove(&file_pb);
                    }
                }

                main_pb.inc(1);
            }
        });

        stream::iter(download_futures)
            .buffer_unordered(ARGS.concurrent_downloads.get())
            .collect::<Vec<()>>()
            .await;

        main_pb.finish_with_message("All downloads complete.");
    }

    async fn write_to_file_chunked_with_progress(
        path: &std::path::Path,
        mut response: reqwest::Response,
        pb: &ProgressBar,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).await?;
        }

        let mut file = File::create(path).await?;
        while let Some(chunk) = response.chunk().await.unwrap() {
            file.write_all(&chunk).await?;
            pb.inc(chunk.len() as u64);
        }

        Ok(())
    }
}
