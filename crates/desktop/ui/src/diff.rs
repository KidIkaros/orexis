//! Diff classification and rendering.

use leptos::prelude::*;

fn diff_class(line: &str) -> &'static str {
    if line.starts_with("+++") || line.starts_with("---") {
        "meta"
    } else if line.starts_with('+') {
        "add"
    } else if line.starts_with('-') {
        "del"
    } else if line.starts_with("@@") {
        "hunk"
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
    let left = text
        .lines()
        .filter(|line| !line.starts_with('+'))
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let right = text
        .lines()
        .filter(|line| !line.starts_with('-'))
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    view! {
        <div class="diff-split">
            <pre class="diff">{left}</pre>
            <pre class="diff">{right}</pre>
        </div>
    }
    .into_any()
}
