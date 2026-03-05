use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/llvm-ir-explorer.css"/>

        // sets the document title
        <Title text="LLVM IR Explorer"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                    <Route path="/*any" view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

use crate::components::*;

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let code = create_rw_signal(String::from("int foo(int x) {\n    return x + 0;\n}"));
    let opt_level = create_rw_signal(String::from("O2"));
    
    let active_tab = create_rw_signal(String::from("LLVM IR"));
    let (llvm_ir, set_llvm_ir) = create_signal(String::new());
    let (optimized_ir, set_optimized_ir) = create_signal(String::new());
    let (assembly_content, set_assembly_content) = create_signal(String::new());
    let (cfgs, set_cfgs) = create_signal::<Vec<crate::server_functions::Cfg>>(vec![]);
    
    let (passes, set_passes) = create_signal::<Vec<crate::server_functions::OptimizationPass>>(vec![]);
    let current_pass_index = create_rw_signal(0usize);

    let (error_msg, set_error_msg) = create_signal::<Option<String>>(None);

    let compile_action = create_server_action::<crate::server_functions::CompileAndOptimize>();

    // React to compilation results
    create_effect(move |_| {
        match compile_action.value().get() {
            Some(Ok(res)) => {
                set_error_msg.set(None);
                set_llvm_ir.set(res.initial_ir);
                set_assembly_content.set(res.assembly);
                set_passes.set(res.passes);
                set_cfgs.set(res.cfgs);
                current_pass_index.set(0);
            }
            Some(Err(e)) => {
                set_error_msg.set(Some(e.to_string()));
                set_llvm_ir.set(String::new());
                set_assembly_content.set(String::new());
                set_passes.set(vec![]);
                set_cfgs.set(vec![]);
                current_pass_index.set(0);
            }
            None => {}
        }
    });

    // Update optimized IR view when slider moves
    create_effect(move |_| {
        let p = passes.get();
        let idx = current_pass_index.get();
        if let Some(pass) = p.get(idx) {
            set_optimized_ir.set(pass.ir.clone());
        } else {
            set_optimized_ir.set(String::new());
        }
    });

    let left_width_px = create_rw_signal(480i32);
    let is_resizing = create_rw_signal(false);

    view! {
        <div class="app-container">
            <header>
                <h1>"LLVM IR Explorer"</h1>
            </header>
            <div class="content-split"
                on:mousemove=move |e: leptos::ev::MouseEvent| {
                    if is_resizing.get() {
                        left_width_px.set(e.client_x().max(220).min(1600));
                    }
                }
                on:mouseup=move |_| is_resizing.set(false)
                on:mouseleave=move |_| is_resizing.set(false)
            >
                <div class="left-panel" style=move || format!("flex: 0 0 {}px;", left_width_px.get())>
                    <CodeEditor 
                        code=code
                        opt_level=opt_level
                        on_compile=compile_action
                        is_pending=compile_action.pending()
                    />
                    <Timeline
                        passes=Signal::derive(move || passes.get().iter().map(|p| p.name.clone()).collect::<Vec<_>>())
                        current_pass_index=current_pass_index
                    />
                </div>
                <div class="resize-handle"
                    on:mousedown=move |e: leptos::ev::MouseEvent| {
                        e.prevent_default();
                        is_resizing.set(true);
                    }
                ></div>
                <div class="right-panel">
                    <OutputTabs
                        active_tab=active_tab
                        llvm_ir=llvm_ir
                        optimized_ir=optimized_ir
                        assembly_content=assembly_content
                        cfgs=cfgs
                        error=error_msg
                    />
                </div>
            </div>
        </div>
    }
}

/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    // set an HTTP status code 404
    // this is feature gated because it can only be done during
    // initial server-side rendering
    // if you navigate to the 404 page subsequently, the status
    // code will not be set because there is not a new HTTP request
    // to the server
    #[cfg(feature = "ssr")]
    {
        // this can be done inline because it's synchronous
        // if it were async, we'd use a server function
        let resp = expect_context::<leptos_axum::ResponseOptions>();
        resp.set_status(axum::http::StatusCode::NOT_FOUND);
    }

    view! {
        <h1>"Not Found"</h1>
    }
}
