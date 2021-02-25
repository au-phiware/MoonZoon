use std::future::Future;
use std::error::Error;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::task;
use tokio::sync::oneshot;
use tokio::signal;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio::time;
use warp::Filter;
use warp::http;
use warp::sse::Event;
use warp::host::Authority;
use warp::path::FullPath;
use uuid::Uuid;
use std::process::exit;
use std::time::Duration;
use std::thread;

pub struct Frontend {
    title: String,
}

impl Frontend {
    pub fn new() -> Self {
        Self {
            title: String::new()
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

pub struct UpMsgRequest {

}

#[macro_export]
macro_rules! start {
    ($init:expr, $frontend:expr, $up_msg_handler:expr) => {
        $crate::start($init, $frontend, $up_msg_handler);
    };
}

pub fn start<IN, FR, UP>(
    init: impl FnOnce() -> IN, 
    frontend: impl Fn() -> FR + Copy + Send + Sync + 'static, 
    up_msg_handler: impl Fn(UpMsgRequest) -> UP + Copy + Send + Sync + 'static,
) -> Result<(), Box<dyn Error>>
where
    IN: Future<Output = ()>,
    FR: Future<Output = Frontend> + Send,
    UP: Future<Output = ()> + Send,
{
    // ctrlc::set_handler(|| {
    //     println!("moon A ctrcl handler triggered");
    //     exit(0);
    // }).unwrap();

    let rt  = Runtime::new()?;
    rt.block_on(async move {
        let sse_senders = Vec::<mpsc::UnboundedSender<Result<Event, Infallible>>>::new();
        let sse_senders = Arc::new(Mutex::new(sse_senders));
        let sse_senders = warp::any().map(move || sse_senders.clone());

        let backend_id = Uuid::new_v4();

        init().await;

        let api = warp::post().and(warp::path("api"));

        let up_msg_handler_route = api
            .and(warp::path("up_msg_handler"))
            .and_then(move || async move {
                up_msg_handler(UpMsgRequest {}).await;
                Ok::<_, warp::Rejection>(http::StatusCode::OK)
            });

        let reload = api
            .and(warp::path("reload"))
            .and(sse_senders.clone())
            .map(|sse_senders: Arc<Mutex<Vec<mpsc::UnboundedSender<Result<Event, Infallible>>>>>| {
                sse_senders.lock().unwrap().retain(|sse_sender| {
                    sse_sender.send(Ok(Event::default().event("reload").data(""))).is_ok()
                });
                http::StatusCode::OK
            });

        let sse = warp::path!("sse")
            .and(sse_senders)
            .map(move |sse_senders: Arc<Mutex<Vec<mpsc::UnboundedSender<Result<Event, Infallible>>>>>| {
                let (sse_sender, sse_receiver) = mpsc::unbounded_channel();
                let sse_stream = UnboundedReceiverStream::<Result<Event, Infallible>>::new(sse_receiver);

                let backend_id = backend_id.to_simple_ref().to_string();
                sse_sender.send(Ok(Event::default().event("backend_id").data(backend_id))).unwrap();

                sse_senders.lock().unwrap().push(sse_sender);
                warp::sse::reply(warp::sse::keep_alive().stream(sse_stream))
            });

        let pkg_route = warp::path("pkg").and(warp::fs::dir("frontend/pkg/"));

        let frontend_route = warp::get().and_then(move || async move {
            let frontend = frontend().await;
            Ok::<_, warp::Rejection>(warp::reply::html(html(&frontend.title)))
        });
        
        let https_routes = up_msg_handler_route
            .or(reload)
            .or(sse)
            .or(pkg_route)
            .or(frontend_route);
            
        let http_routes = warp::host::optional()
            .map(|authority: Option<Authority>| {
                // println!("Host + port: {:#?}", authority);
                // println!("Path: {:#?}", warp::path::full().to_string());
                // path = warp::path::full();
                // warp::redirect(Uri::from_static("https://127.0.0.1:2443"))
                "http"
            });

        let http_port = 8080;
        let https_port = 8443;
            
        let (shutdown_sender_http, shutdown_receiver_http) = oneshot::channel();
        let (_, http_server) = warp::serve(http_routes)
            .bind_with_graceful_shutdown(([0, 0, 0, 0], http_port), async {
                shutdown_receiver_http.await.ok();
            });
        let http_server_handle = task::spawn(http_server);

        let (shutdown_sender_https, shutdown_receiver_https) = oneshot::channel();
        let (_, https_server) = warp::serve(https_routes)
            .tls()
            .cert_path("backend/private/public.pem")
            .key_path("backend/private/private.pem")
            .bind_with_graceful_shutdown(([0, 0, 0, 0], https_port), async {
                shutdown_receiver_https.await.ok();
            });
        let https_server_handle = task::spawn(https_server);

        println!("HTTP server is running on 0.0.0.0:{port} and redirects to HTTPS", port = http_port);
        println!("HTTPS server is running on 0.0.0.0:{port} [https://127.0.0.1:{port}]", port = https_port);

        signal::ctrl_c().await.unwrap();
        shutdown_sender_http.send(()).unwrap();
        shutdown_sender_https.send(()).unwrap();
        // time::sleep(time::Duration::from_secs(1)).await;
        http_server_handle.abort();
        https_server_handle.abort();
        futures::future::join_all(vec![http_server_handle, https_server_handle]).await;
        println!("Moon shut down");
    });
    Ok(())
}

fn html(title: &str) -> String {
    format!(r#"<!DOCTYPE html>
    <html lang="en">
    
    <head>
      <meta charset="utf-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no" />
      <title>{title}</title>
    </head>

    <body>
      <h1>MoonZoon is running!</h1>
      <h2>A random uuid: {uuid}</h2>
      <section id="app"></section>

      <script type="text/javascript">
        {reconnecting_event_source}
        var uri = location.protocol + '//' + location.host + '/sse';
        var sse = new ReconnectingEventSource(uri);
        var backendId = null;
        sse.addEventListener("backend_id", function(msg) {{
            var newBackendId = msg.data;
            if(backendId === null) {{
                backendId = newBackendId;
            }} else if(backendId !== newBackendId) {{
                sse.close();
                location.reload();
            }}
          }});
        sse.addEventListener("reload", function(msg) {{
          sse.close();
          location.reload();
        }});
      </script>

      <script type="module">
        import init from '/pkg/frontend.js';
        init('/pkg/frontend_bg.wasm');
      </script>
    </body>
    
    </html>"#, 
    title = title, 
    uuid = Uuid::new_v4().to_simple_ref(), 
    reconnecting_event_source = include_str!("../js/ReconnectingEventSource.min.js"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
