use leptos::*;

#[component]
pub fn CodeEditor(
    code: RwSignal<String>,
    opt_level: RwSignal<String>,
    on_compile: Action<
        crate::server_functions::CompileAndOptimize,
        Result<crate::server_functions::CompileResult, ServerFnError>,
    >,
    is_pending: ReadSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="code-editor-panel">
            <div class="header">
                <h2>"C/C++ Source"</h2>
                <select
                    on:change=move |ev| opt_level.set(event_target_value(&ev))
                    prop:value=opt_level
                >
                    <option value="O0">"O0"</option>
                    <option value="O1">"O1"</option>
                    <option value="O2">"O2"</option>
                    <option value="O3">"O3"</option>
                </select>
                <button
                    disabled=move || is_pending.get()
                    on:click=move |_| {
                        on_compile.dispatch(crate::server_functions::CompileAndOptimize {
                            code: code.get(),
                            opt_level: opt_level.get(),
                        });
                    }
                >
                    {move || if is_pending.get() { "COMPILING..." } else { "COMPILE" }}
                </button>
            </div>
            <textarea
                class="code-input"
                on:input=move |ev| code.set(event_target_value(&ev))
                prop:value=code
                placeholder="Paste C/C++ here..."
            />
        </div>
    }
}

#[component]
pub fn OutputTabs(
    active_tab: RwSignal<String>,
    llvm_ir: ReadSignal<String>,
    optimized_ir: ReadSignal<String>,
    assembly_content: ReadSignal<String>,
    cfgs: ReadSignal<Vec<crate::server_functions::Cfg>>,
    error: ReadSignal<Option<String>>,
) -> impl IntoView {
    let tabs = vec!["LLVM IR", "Optimized IR", "IR Diff", "CFG", "Assembly"];

    let hl_ir  = create_memo(move |_| crate::highlight::highlight_ir(&llvm_ir.get()));
    let hl_opt = create_memo(move |_| crate::highlight::highlight_ir(&optimized_ir.get()));
    let hl_asm = create_memo(move |_| crate::highlight::highlight_asm(&assembly_content.get()));
    let diff_pair = create_memo(move |_| {
        crate::highlight::diff_ir(&llvm_ir.get(), &optimized_ir.get())
    });

    create_effect(move |_| {
        if error.get().is_some() {
            active_tab.set("LLVM IR".to_string());
        }
    });

    view! {
        <div class="output-panel">
            <Show when=move || error.get().is_some() fallback=|| ()>
                <div class="error-banner">
                    <span class="error-tag">"! ERROR"</span>
                    <pre class="error-text">{move || error.get().unwrap_or_default()}</pre>
                </div>
            </Show>

            <div class="tab-header">
                {tabs.into_iter().map(|tab| {
                    let tab_name = tab.to_string();
                    let tab_clone = tab_name.clone();
                    let tab_cmp   = tab_name.clone();
                    let cls = move || if active_tab.get() == tab_cmp { "tab active" } else { "tab" };
                    view! {
                        <button class=cls on:click=move |_| active_tab.set(tab_clone.clone())>
                            {tab_name}
                        </button>
                    }
                }).collect_view()}
            </div>

            <div class="tab-content">
                <Show when=move || active_tab.get() == "LLVM IR" fallback=|| ()>
                    <div class="tab-toolbar">
                        <a class="download-btn"
                            href=move || crate::highlight::to_data_uri(&llvm_ir.get())
                            download="output.ll"
                        >"↓ .ll"</a>
                    </div>
                    <pre class="code-output" inner_html=move || hl_ir.get()></pre>
                </Show>

                <Show when=move || active_tab.get() == "Optimized IR" fallback=|| ()>
                    <div class="tab-toolbar">
                        <a class="download-btn"
                            href=move || crate::highlight::to_data_uri(&optimized_ir.get())
                            download="optimized.ll"
                        >"↓ .ll"</a>
                    </div>
                    <pre class="code-output" inner_html=move || hl_opt.get()></pre>
                </Show>

                <Show when=move || active_tab.get() == "IR Diff" fallback=|| ()>
                    <div class="ir-diff-view">
                        <div class="ir-diff-pane">
                            <h4 class="diff-pane-title">"INITIAL IR (O0)"</h4>
                            <pre class="code-output"
                                inner_html=move || diff_pair.get().0></pre>
                        </div>
                        <div class="ir-diff-pane">
                            <h4 class="diff-pane-title">"OPTIMIZED (CURRENT PASS)"</h4>
                            <pre class="code-output"
                                inner_html=move || diff_pair.get().1></pre>
                        </div>
                    </div>
                </Show>

                <Show when=move || active_tab.get() == "Assembly" fallback=|| ()>
                    <div class="tab-toolbar">
                        <a class="download-btn"
                            href=move || crate::highlight::to_data_uri(&assembly_content.get())
                            download="output.s"
                        >"↓ .s"</a>
                    </div>
                    <pre class="code-output" inner_html=move || hl_asm.get()></pre>
                </Show>

                <Show when=move || active_tab.get() == "CFG" fallback=|| ()>
                    <div class="cfg-view">
                        <For
                            each=move || cfgs.get().into_iter()
                            key=|cfg| cfg.function_name.clone()
                            children=move |cfg| view! {
                                <div class="cfg-function">
                                    <h3 class="cfg-fn-title">"FN: " {cfg.function_name.clone()}</h3>
                                    <div class="cfg-svg" inner_html=cfg.dot_content.clone()></div>
                                </div>
                            }
                        />
                    </div>
                </Show>
            </div>
        </div>
    }
}

#[component]
pub fn Timeline(
    #[prop(into)] passes: Signal<Vec<String>>,
    current_pass_index: RwSignal<usize>,
) -> impl IntoView {
    let filter = create_rw_signal(String::new());

    view! {
        <div class="timeline-visualizer">
            <div class="timeline-header">
                <h3>"OPTIMIZATION PIPELINE"</h3>
                <span class="pass-counter">
                    {move || {
                        let total = passes.with(|p| p.len());
                        let idx   = current_pass_index.get();
                        if total > 0 { format!("[{}/{}]", idx + 1, total) }
                        else         { "[0/0]".to_string() }
                    }}
                </span>
            </div>

            <div class="timeline-slider">
                <input
                    type="range"
                    min="0"
                    max=move || {
                        let len = passes.with(|p| p.len());
                        if len > 0 { len - 1 } else { 0 }
                    }
                    prop:value=move || current_pass_index.get()
                    on:input=move |ev| {
                        if let Ok(val) = event_target_value(&ev).parse::<usize>() {
                            current_pass_index.set(val);
                        }
                    }
                />
            </div>

            <input
                class="pass-filter"
                type="text"
                placeholder="FILTER PASSES..."
                on:input=move |ev| filter.set(event_target_value(&ev))
                prop:value=filter
            />

            <div class="passes-list">
                <For
                    each=move || {
                        let f = filter.get().to_lowercase();
                        passes
                            .get()
                            .into_iter()
                            .enumerate()
                            .filter(|(_, name)| {
                                f.is_empty() || name.to_lowercase().contains(&f)
                            })
                            .collect::<Vec<_>>()
                    }
                    key=|(i, _)| *i
                    children=move |(i, name)| {
                        let cls = move || {
                            if i == current_pass_index.get() { "pass active" } else { "pass" }
                        };
                        view! {
                            <div class=cls on:click=move |_| current_pass_index.set(i)>
                                {name}
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}
