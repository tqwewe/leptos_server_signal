#[cfg(feature = "ssr")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    use actix_example::app::*;
    use actix_files::Files;
    use actix_web::*;
    use leptos::*;
    use leptos_actix::{generate_route_list, LeptosRoutes};

    let conf = get_configuration(None).await.unwrap();
    let addr = conf.leptos_options.site_addr;
    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(|cx| view! { cx, <App/> });

    HttpServer::new(move || {
        let leptos_options = &conf.leptos_options;
        let site_root = &leptos_options.site_root;

        App::new()
            .route("/api/{tail:.*}", leptos_actix::handle_server_fns())
            .route("/ws", web::get().to(websocket))
            .leptos_routes(
                leptos_options.to_owned(),
                routes.to_owned(),
                |cx| view! { cx, <App/> },
            )
            .service(Files::new("/", site_root))
        //.wrap(middleware::Compress::default())
    })
    .bind(&addr)?
    .run()
    .await
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}

#[cfg(feature = "ssr")]
pub async fn websocket(
    req: actix_web::HttpRequest,
    stream: actix_web::web::Payload,
) -> impl actix_web::Responder {
    use std::time::Duration;

    use actix_example::app::Count;
    use leptos_server_signal::ServerSignal;

    let (res, session, _msg_stream) = actix_ws::handle(&req, stream).unwrap();
    let mut count = ServerSignal::<Count>::new("counter", session).unwrap();

    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(Duration::from_millis(100)).await;
            let result = count.with(|count| count.value += 1).await;
            if result.is_err() {
                break;
            }
        }
    });

    res
}
