use bytes::BufMut;
use futures::TryStreamExt;
use std::path::PathBuf;
use tokio_stream::StreamExt;
use warp::{
    hyper::{Method, StatusCode},
    multipart::FormData,
    reply, Filter, Rejection, Reply,
}; // for stream ops,

mod filters;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().filter_or("LOG", "info")).init();

    let pool = db_client::connect()
        .await
        .expect("Failed to connect to database.");

    let api = filters::app_api(pool.clone());
    let api = warp::path("api").and(api).boxed();
    let upload_cors = warp::cors()
        .allow_any_origin()
        .allow_methods(&[Method::POST]);
    let upload = warp::path("upload")
        .and(warp::post())
        .and(warp::multipart::form().max_length(5_000_000))
        .and_then(upload)
        .with(upload_cors);
    let graphiql = warp::path("graphiql")
        .and(juniper_warp::graphiql_filter("/api", None))
        .boxed();

    warp::serve(upload.or(api).or(graphiql).with(warp::log("http")))
        .run(([0, 0, 0, 0], 3030))
        .await;
}

async fn upload(mut form: FormData) -> Result<impl Reply, Rejection> {
    while let Some(Ok(part)) = form.next().await {
        let part_name = part.name();
        let part_content_type = part.content_type();
        println!("{},{:?}", &part_name, &part_content_type);
        if part_name == "file" {
            let uuid = uuid::Uuid::new_v4();
            let mut file_path = PathBuf::from(std::env::current_dir().unwrap());
            file_path.push("filestore");
            file_path.push(uuid.to_string());
            file_path.set_extension("png");

            println!("{:?}", file_path);

            // let mut file = match OpenOptions::new()
            //     .create(true)
            //     .write(true)
            //     .open(file_path)
            //     .await
            // {
            //     Ok(file) => file,
            //     Err(error) => {
            //         println!("{:?}", error);
            //         return Ok(reply::with_status("", StatusCode::INTERNAL_SERVER_ERROR));
            //     }
            // };

            println!("part_stream");

            let mut part_stream = part.stream();

            let value = part_stream
                .try_fold(Vec::new(), |mut vec, data| {
                    vec.put(data);
                    async move { Ok(vec) }
                })
                .await
                .map_err(|e| {
                    eprintln!("reading file error: {}", e);
                    warp::reject::reject()
                })?;

            tokio::fs::write(&file_path, value).await.map_err(|e| {
                eprint!("error writing file: {}", e);
                warp::reject::reject()
            })?;
        }
    }

    Ok(reply::with_status("Nice", StatusCode::OK))
}
