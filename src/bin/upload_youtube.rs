use std::fs;

// use google_youtube3::{
//     api::{Video, VideoSnippet, VideoStatus},
//     hyper, hyper_rustls, oauth2, YouTube,
// };
use hoti_rs::{scp::SCP, ContentSource};
use reqwest_middleware::ClientBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get an ApplicationSecret instance by some means. It contains the `client_id` and
    // `client_secret`, among other things.
    // let secret: oauth2::ApplicationSecret = oauth2::ApplicationSecret {
    //     client_id: "386071646136-97ohe87asvm0e1qgnemkb907mp9g23fv.apps.googleusercontent.com"
    //         .into(),
    //     client_secret: "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".into(),
    //     auth_uri: "https://accounts.google.com/o/oauth2/auth".into(),
    //     token_uri: "https://accounts.google.com/o/oauth2/token".into(),
    //     ..Default::default()
    // };
    // // Instantiate the authenticator. It will choose a suitable authentication flow for you,
    // // unless you replace  `None` with the desired Flow.
    // // Provide your own `AuthenticatorDelegate` to adjust the way it operates and get feedback about
    // // what's going on. You probably want to bring in your own `TokenStorage` to persist tokens and
    // // retrieve them from storage.
    // let auth = oauth2::InstalledFlowAuthenticator::builder(
    //     secret,
    //     oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    // )
    // .build()
    // .await
    // .unwrap();

    // let hub = YouTube::new(
    //     hyper::Client::builder().build(
    //         hyper_rustls::HttpsConnectorBuilder::new()
    //             .with_native_roots()
    //             .https_or_http()
    //             .enable_http1()
    //             .enable_http2()
    //             .build(),
    //     ),
    //     auth,
    // );

    let retry_policy =
        reqwest_retry::policies::ExponentialBackoff::builder().build_with_max_retries(5);
    let reqwest = ClientBuilder::new(reqwest::Client::new())
        .with(reqwest_retry::RetryTransientMiddleware::new_with_policy(
            retry_policy,
        ))
        .build();

    for (idx, scp) in SCP::iter()?.enumerate().skip(4) {
        let name = scp.name().to_ascii_uppercase();
        let Ok(file) = fs::File::open(format!("{name}.mp4")) else {
            break;
        };

        let title = scp.title(reqwest.clone()).await.unwrap_or("Unknown".into());

        println!("{name}: {title} | Summarized");
        println!(
            "#shorts #scp #mystery #fiction #horror #summary\nFull SCP: {}",
            scp.url()
        );
        println!("")

        // As the method needs a request, you would usually fill it with the desired information
        // into the respective structure. Some of the parts shown here might not be applicable !
        // Values shown here are possibly random and not representative !
        // let req = Video {
        //     snippet: Some(VideoSnippet {
        //         title: Some(format!("{name}: {title} | Summarized")),
        //         description: Some(format!(
        //             "#shorts #scp #mystery #fiction #horror #summary\nFull SCP: {}",
        //             scp.url()
        //         )),
        //         tags: Some(vec![
        //             "shorts".into(),
        //             "scp".into(),
        //             "mystery".into(),
        //             "fiction".into(),
        //             "horror".into(),
        //             "summary".into(),
        //         ]),
        //         ..Default::default()
        //     }),
        //     status: Some(VideoStatus {
        //         privacy_status: Some("public".into()),
        //         made_for_kids: Some(false),
        //         ..Default::default()
        //     }),
        //     ..Default::default()
        // };

        // // You can configure optional parameters by calling the respective setters at will, and
        // // execute the final call using `upload(...)`.
        // // Values shown here are possibly random and not representative !
        // let _ = hub
        //     .videos()
        //     .insert(req)
        //     .stabilize(false)
        //     .notify_subscribers(true)
        //     .auto_levels(true)
        //     .upload(file, "video/mp4".parse().unwrap())
        //     .await?;

        // println!("{}: Uploaded {name}", idx + 1);
    }

    Ok(())
}
