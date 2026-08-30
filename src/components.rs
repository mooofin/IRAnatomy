use leptos::*;

#[component]
pub fn CodeEditor(
    code: RwSignal<String>,
    opt_level: RwSignal<String>,
    language: RwSignal<String>,
    extra_flags: RwSignal<String>,
    custom_pipeline: RwSignal<String>,
    on_compile: Action<
        crate::server_functions::CompileAndOptimize,
        Result<crate::server_functions::CompileResult, ServerFnError>,
    >,
    is_pending: ReadSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="code-editor-panel">
            <div class="header">
                <h2>"Source"</h2>
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
                            language: language.get(),
                            extra_flags: extra_flags.get(),
                            custom_pipeline: custom_pipeline.get(),
                        });
                    }
                >
                    {move || if is_pending.get() { "COMPILING..." } else { "COMPILE" }}
                </button>
            </div>
            <div class="editor-flags-row">
                <select
                    class="lang-select"
                    title="Source language"
                    on:change=move |ev| language.set(event_target_value(&ev))
                    prop:value=language
                >
                    <option value="cpp">"C++"</option>
                    <option value="c">"C"</option>
                </select>
                <input
                    class="flag-input"
                    type="text"
                    placeholder="CLANG FLAGS: e.g. -std=c++20 -march=native -ffast-math"
                    title="Extra clang flags applied to both IR and assembly generation"
                    spellcheck="false"
                    on:input=move |ev| extra_flags.set(event_target_value(&ev))
                    prop:value=extra_flags
                />
                <input
                    class="pipeline-input"
                    type="text"
                    placeholder="OPT PIPELINE: empty = default<Ox>"
                    title="Custom opt pass pipeline, e.g. mem2reg,instcombine"
                    spellcheck="false"
                    on:input=move |ev| custom_pipeline.set(event_target_value(&ev))
                    prop:value=custom_pipeline
                />
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
    passes: ReadSignal<Vec<crate::server_functions::OptimizationPass>>,
    current_pass_index: RwSignal<usize>,
    diff_mode: RwSignal<String>,
    error: ReadSignal<Option<String>>,
) -> impl IntoView {
    let tabs = vec!["LLVM IR", "Optimized IR", "IR Diff", "CFG", "Assembly"];

    let hl_ir  = create_memo(move |_| crate::highlight::highlight_ir(&llvm_ir.get()));
    let hl_opt = create_memo(move |_| crate::highlight::highlight_ir(&optimized_ir.get()));
    let hl_asm = create_memo(move |_| crate::highlight::highlight_asm(&assembly_content.get()));
    let diff_pair = create_memo(move |_| {
        let idx = current_pass_index.get();
        let (old, new) = if diff_mode.get() == "previous" {
            let current_passes = passes.get();
            let new_ir = current_passes
                .get(idx)
                .map(|p| p.ir.clone())
                .unwrap_or_default();
            let old_ir = if idx == 0 {
                llvm_ir.get()
            } else {
                current_passes
                    .get(idx - 1)
                    .map(|p| p.ir.clone())
                    .unwrap_or_default()
            };
            (old_ir, new_ir)
        } else {
            (llvm_ir.get(), optimized_ir.get())
        };
        crate::highlight::diff_ir(&old, &new)
    });
    let diff_old_title = move || {
        if diff_mode.get() == "previous" && current_pass_index.get() > 0 {
            "PREVIOUS PASS"
        } else {
            "INITIAL IR (O0)"
        }
    };
    let diff_new_title = move || {
        if diff_mode.get() == "previous" {
            "CURRENT PASS"
        } else {
            "OPTIMIZED (CURRENT PASS)"
        }
    };

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
                    <div class="ir-diff-tab">
                        <div class="diff-mode-toggle">
                            <button
                                class=move || {
                                    if diff_mode.get() == "baseline" {
                                        "diff-toggle-btn active"
                                    } else {
                                        "diff-toggle-btn"
                                    }
                                }
                                on:click=move |_| diff_mode.set("baseline".to_string())
                            >
                                "VS O0 BASELINE"
                            </button>
                            <button
                                class=move || {
                                    if diff_mode.get() == "previous" {
                                        "diff-toggle-btn active"
                                    } else {
                                        "diff-toggle-btn"
                                    }
                                }
                                on:click=move |_| diff_mode.set("previous".to_string())
                            >
                                "VS PREVIOUS PASS"
                            </button>
                        </div>
                        <div class="ir-diff-view">
                            <div class="ir-diff-pane">
                                <h4 class="diff-pane-title">{diff_old_title}</h4>
                                <pre class="code-output"
                                    inner_html=move || diff_pair.get().0></pre>
                            </div>
                            <div class="ir-diff-pane">
                                <h4 class="diff-pane-title">{diff_new_title}</h4>
                                <pre class="code-output"
                                    inner_html=move || diff_pair.get().1></pre>
                            </div>
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
