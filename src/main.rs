#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use llvm_ir_explorer::app::*;
    use llvm_ir_explorer::server_functions::*;
    use tower_http::services::ServeDir;

    let conf = get_configuration(Some("Cargo.toml")).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // Explicitly register server functions
    server_fn::axum::register_explicit::<CompileAndOptimize>();

    // Resolve site_root to absolute path
    let site_root = std::env::current_dir()
        .unwrap()
        .join(&leptos_options.site_root);
    println!("Serving static files from: {:?}", site_root);

    let app = Router::new()
        .route("/api/*fn_name", axum::routing::get(leptos_axum::handle_server_fns).post(leptos_axum::handle_server_fns))
        // Serve pkg (JS/WASM/CSS) BEFORE leptos_routes so the /*any wildcard doesn't intercept them
        .nest_service("/pkg", ServeDir::new(site_root.join("pkg")))
        .leptos_routes(&leptos_options, routes, App)
        .fallback_service(ServeDir::new(&site_root))
        .with_state(leptos_options);

    println!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
}
