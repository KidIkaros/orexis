//! Diff classification and hunk-aligned split rendering.

use leptos::prelude::*;

fn diff_class(line: &str) -> &'static str {
    if line.starts_with("+++")
        || line.starts_with("---")
        || line.starts_with("diff --git")
        || line.starts_with("index ")
    {
        "meta"
    } else if line.starts_with("@@") {
        "hunk"
    } else if line.starts_with('+') {
        "add"
    } else if line.starts_with('-') {
        "del"
    } else {
        "ctx"
    }
}

pub fn unified_diff(text: String) -> AnyView {
    let lines = text
        .lines()
        .map(|line| {
            let class = diff_class(line);
            let line = line.to_string();
            view! { <span class=class>{line}"\n"</span> }
        })
        .collect_view();
    view! { <pre class="diff">{lines}</pre> }.into_any()
}

pub fn split_diff(text: String) -> AnyView {
    let raw_lines: Vec<&str> = text.lines().collect();
    let mut left_spans = Vec::new();
    let mut right_spans = Vec::new();
    let mut index = 0;

    while index < raw_lines.len() {
        let line = raw_lines[index];
        if line.starts_with("---")
            && index + 1 < raw_lines.len()
            && raw_lines[index + 1].starts_with("+++")
        {
            let l_text = line.to_string();
            let r_text = raw_lines[index + 1].to_string();
            left_spans.push(view! { <span class="meta">{l_text}"\n"</span> });
            right_spans.push(view! { <span class="meta">{r_text}"\n"</span> });
            index += 2;
            continue;
        }

        let class = diff_class(line);
        if class == "del" || class == "add" {
            let mut dels = Vec::new();
            let mut adds = Vec::new();
            while index < raw_lines.len() {
                let current = raw_lines[index];
                let c_class = diff_class(current);
                if c_class == "del" {
                    dels.push(current.to_string());
                    index += 1;
                } else if c_class == "add" {
                    adds.push(current.to_string());
                    index += 1;
                } else {
                    break;
                }
            }
            let count = dels.len().max(adds.len());
            for i in 0..count {
                match dels.get(i) {
                    Some(l) => {
                        let l = l.clone();
                        left_spans.push(view! { <span class="del">{l}"\n"</span> });
                    }
                    None => {
                        left_spans.push(view! { <span class="pad">{String::new()}"\n"</span> });
                    }
                }
                match adds.get(i) {
                    Some(r) => {
                        let r = r.clone();
                        right_spans.push(view! { <span class="add">{r}"\n"</span> });
                    }
                    None => {
                        right_spans.push(view! { <span class="pad">{String::new()}"\n"</span> });
                    }
                }
            }
        } else {
            let l = line.to_string();
            let r = line.to_string();
            left_spans.push(view! { <span class=class>{l}"\n"</span> });
            right_spans.push(view! { <span class=class>{r}"\n"</span> });
            index += 1;
        }
    }

    let left_view = left_spans.collect_view();
    let right_view = right_spans.collect_view();
    view! {
        <div class="diff-split">
            <pre class="diff">{left_view}</pre>
            <pre class="diff">{right_view}</pre>
        </div>
    }
    .into_any()
}
