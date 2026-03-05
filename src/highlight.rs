pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}


pub fn highlight_ir(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(text.len() * 2);
    for line in text.lines() {
        out.push_str(&highlight_ir_line(line));
        out.push('\n');
    }
    out
}

pub fn highlight_ir_line(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with(';') {
        return format!("<span class='hl-comment'>{}</span>", escape_html(line));
    }
    if is_ir_label(trimmed) {
        return format!("<span class='hl-label'>{}</span>", escape_html(line));
    }
    tokenize_ir(line)
}

fn is_ir_label(trimmed: &str) -> bool {
    if !trimmed.ends_with(':') {
        return false;
    }
    let label = &trimmed[..trimmed.len() - 1];
    !label.is_empty()
        && !label.contains(' ')
        && label
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-' | '$'))
}

fn tokenize_ir(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(n * 2);
    let mut i = 0;

    while i < n {
        match chars[i] {
            '"' => {
                let start = i;
                i += 1;
                while i < n {
                    if chars[i] == '\\' {
                        i += 1;
                    } else if chars[i] == '"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                out.push_str(&format!("<span class='hl-string'>{}</span>", escape_html(&s)));
            }
            ';' => {
                let rest: String = chars[i..].iter().collect();
                out.push_str(&format!(
                    "<span class='hl-comment'>{}</span>",
                    escape_html(&rest)
                ));
                break;
            }
            '%' | '@' => {
                let pfx = chars[i];
                let start = i;
                i += 1;
                if i < n && chars[i] == '"' {
                    i += 1;
                    while i < n && chars[i] != '"' {
                        i += 1;
                    }
                    if i < n {
                        i += 1;
                    }
                } else {
                    while i < n && is_ir_id(chars[i]) {
                        i += 1;
                    }
                }
                let s: String = chars[start..i].iter().collect();
                let cls = if pfx == '@' { "hl-global" } else { "hl-value" };
                out.push_str(&format!("<span class='{}'>{}</span>", cls, escape_html(&s)));
            }
            '!' => {
                let start = i;
                i += 1;
                while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                out.push_str(&format!("<span class='hl-meta'>{}</span>", escape_html(&s)));
            }
            '#' => {
                let start = i;
                i += 1;
                while i < n && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                if s.len() > 1 {
                    out.push_str(&format!("<span class='hl-attr'>{}</span>", escape_html(&s)));
                } else {
                    out.push_str(&escape_html(&s));
                }
            }
            '-' if i + 1 < n && chars[i + 1].is_ascii_digit() => {
                let start = i;
                i += 1;
                while i < n && is_num_char(chars[i]) {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                out.push_str(&format!("<span class='hl-number'>{}</span>", escape_html(&s)));
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < n && is_num_char(chars[i]) {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                out.push_str(&format!("<span class='hl-number'>{}</span>", escape_html(&s)));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < n
                    && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '_' | '.'))
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                if is_ir_type(&word) {
                    out.push_str(&format!(
                        "<span class='hl-type'>{}</span>",
                        escape_html(&word)
                    ));
                } else if is_ir_keyword(&word) {
                    out.push_str(&format!(
                        "<span class='hl-keyword'>{}</span>",
                        escape_html(&word)
                    ));
                } else {
                    out.push_str(&escape_html(&word));
                }
            }
            '&' => {
                out.push_str("&amp;");
                i += 1;
            }
            '<' => {
                out.push_str("&lt;");
                i += 1;
            }
            '>' => {
                out.push_str("&gt;");
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

fn is_ir_id(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '$')
}

fn is_num_char(c: char) -> bool {
    c.is_ascii_digit()
        || matches!(c, '.' | 'e' | 'E' | 'x' | 'X' | 'a'..='f' | 'A'..='F')
}

fn is_ir_type(s: &str) -> bool {
    if s.starts_with('i') && s.len() > 1 && s[1..].chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    matches!(
        s,
        "void"
            | "half"
            | "bfloat"
            | "float"
            | "double"
            | "fp128"
            | "x86_fp80"
            | "ppc_fp128"
            | "label"
            | "metadata"
            | "token"
            | "ptr"
            | "opaque"
    )
}

fn is_ir_keyword(s: &str) -> bool {
    matches!(
        s,
        "ret" | "br" | "switch" | "indirectbr" | "invoke" | "resume" | "unreachable"
        | "callbr" | "catchswitch" | "catchret" | "cleanupret"
        | "add" | "fadd" | "sub" | "fsub" | "mul" | "fmul"
        | "udiv" | "sdiv" | "fdiv" | "urem" | "srem" | "frem"
        | "shl" | "lshr" | "ashr" | "and" | "or" | "xor"
        | "alloca" | "load" | "store" | "fence" | "cmpxchg" | "atomicrmw"
        | "getelementptr" | "inbounds"
        | "trunc" | "zext" | "sext" | "fptrunc" | "fpext"
        | "fptoui" | "fptosi" | "uitofp" | "sitofp"
        | "ptrtoint" | "inttoptr" | "bitcast" | "addrspacecast"
        | "icmp" | "fcmp" | "phi" | "select" | "call" | "tail" | "musttail" | "notail"
        | "va_arg" | "landingpad" | "catchpad" | "cleanuppad"
        | "extractelement" | "insertelement" | "shufflevector"
        | "extractvalue" | "insertvalue" | "freeze"
        | "define" | "declare" | "global" | "constant" | "type"
        | "target" | "datalayout" | "triple" | "source_filename" | "module" | "attributes"
        | "nsw" | "nuw" | "exact" | "nnan" | "ninf" | "nsz" | "arcp" | "contract" | "afn"
        | "reassoc" | "fast" | "volatile" | "atomic" | "syncscope"
        | "acquire" | "release" | "acq_rel" | "seq_cst" | "monotonic" | "unordered"
        | "align" | "to" | "from"
        | "private" | "internal" | "available_externally" | "linkonce" | "weak"
        | "common" | "appending" | "extern_weak" | "linkonce_odr" | "weak_odr" | "external"
        | "default" | "hidden" | "protected"
        | "dso_local" | "local_unnamed_addr" | "unnamed_addr"
        | "noinline" | "alwaysinline" | "optnone" | "inlinehint"
        | "noreturn" | "nounwind" | "readnone" | "readonly" | "writeonly"
        | "noalias" | "nocapture" | "nonnull" | "noundef" | "returned"
        | "signext" | "zeroext"
        | "null" | "undef" | "poison" | "zeroinitializer" | "none"
        | "true" | "false"
        | "eq" | "ne" | "ugt" | "uge" | "ult" | "ule" | "sgt" | "sge" | "slt" | "sle"
        | "oeq" | "ogt" | "oge" | "olt" | "ole" | "one" | "ord" | "ueq" | "une" | "uno"
        | "x"
    )
}


pub fn highlight_asm(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(text.len() * 2);
    for line in text.lines() {
        out.push_str(&highlight_asm_line(line));
        out.push('\n');
    }
    out
}

fn highlight_asm_line(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return String::new();
    }
    let indent = &line[..line.len() - trimmed.len()];
    let esc_indent = escape_html(indent);

    if trimmed.starts_with('#') || trimmed.starts_with("//") {
        return format!(
            "{}<span class='hl-comment'>{}</span>",
            esc_indent,
            escape_html(trimmed)
        );
    }
    if trimmed.ends_with(':') && !trimmed.contains(' ') {
        return format!(
            "{}<span class='hl-label'>{}</span>",
            esc_indent,
            escape_html(trimmed)
        );
    }
    if trimmed.starts_with('.') {
        let sp = trimmed
            .find(|c: char| c.is_whitespace())
            .unwrap_or(trimmed.len());
        let dir = &trimmed[..sp];
        let rest = &trimmed[sp..];
        return format!(
            "{}<span class='hl-keyword'>{}</span>{}",
            esc_indent,
            escape_html(dir),
            highlight_asm_operands(rest)
        );
    }
    let sp = trimmed
        .find(|c: char| c.is_whitespace())
        .unwrap_or(trimmed.len());
    let instr = &trimmed[..sp];
    let operands = &trimmed[sp..];
    format!(
        "{}<span class='hl-keyword'>{}</span>{}",
        esc_indent,
        escape_html(instr),
        highlight_asm_operands(operands)
    )
}

fn highlight_asm_operands(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(n * 2);
    let mut i = 0;
    while i < n {
        match chars[i] {
            '#' => {
                let rest: String = chars[i..].iter().collect();
                out.push_str(&format!(
                    "<span class='hl-comment'>{}</span>",
                    escape_html(&rest)
                ));
                break;
            }
            '%' => {
                let start = i;
                i += 1;
                while i < n && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                out.push_str(&format!("<span class='hl-value'>{}</span>", escape_html(&s)));
            }
            '$' => {
                let start = i;
                i += 1;
                while i < n
                    && (chars[i].is_ascii_alphanumeric()
                        || matches!(chars[i], '-' | '_' | 'x' | 'X'))
                {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                out.push_str(&format!("<span class='hl-number'>{}</span>", escape_html(&s)));
            }
            '&' => {
                out.push_str("&amp;");
                i += 1;
            }
            '<' => {
                out.push_str("&lt;");
                i += 1;
            }
            '>' => {
                out.push_str("&gt;");
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}


pub fn diff_ir(old: &str, new: &str) -> (String, String) {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let m = old_lines.len();
    let n = new_lines.len();

    if m > 500 || n > 500 {
        let oh = old_lines
            .iter()
            .map(|l| {
                let mut s = highlight_ir_line(l);
                s.push('\n');
                s
            })
            .collect();
        let nh = new_lines
            .iter()
            .map(|l| {
                let mut s = highlight_ir_line(l);
                s.push('\n');
                s
            })
            .collect();
        return (oh, nh);
    }

    let mut dp = vec![vec![0u16; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            dp[i][j] = if old_lines[i] == new_lines[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut old_html = String::new();
    let mut new_html = String::new();
    let mut i = 0;
    let mut j = 0;

    while i < m || j < n {
        if i < m && j < n && old_lines[i] == new_lines[j] {
            let hl = highlight_ir_line(old_lines[i]);
            old_html.push_str(&hl);
            old_html.push('\n');
            new_html.push_str(&hl);
            new_html.push('\n');
            i += 1;
            j += 1;
        } else if j < n && (i >= m || dp[i][j + 1] >= dp[i + 1][j]) {
            new_html.push_str("<span class='diff-add'>");
            new_html.push_str(&highlight_ir_line(new_lines[j]));
            new_html.push_str("</span>\n");
            j += 1;
        } else {
            old_html.push_str("<span class='diff-remove'>");
            old_html.push_str(&highlight_ir_line(old_lines[i]));
            old_html.push_str("</span>\n");
            i += 1;
        }
    }

    (old_html, new_html)
}
