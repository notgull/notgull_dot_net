// GNU AGPL v3 License

use comrak::{
    markdown_to_html_with_plugins, plugins::syntect::SyntectAdapter, ComrakExtensionOptions,
    ComrakOptions, ComrakParseOptions, ComrakPlugins, ComrakRenderOptions, ComrakRenderPlugins,
};
use once_cell::sync::OnceCell;

#[inline]
pub fn markdown(input: &str) -> String {
    let plugins = comrak_plugins();
    markdown_to_html_with_plugins(input, &COMRAK_OPTIONS, &plugins)
}

#[inline]
pub fn initialize_markdown() {
    let _ = SYNTECT_ADAPTOR.set(SyntectAdapter::new("base16-ocean.dark"));
}

#[inline]
fn comrak_plugins() -> ComrakPlugins<'static> {
    ComrakPlugins {
        render: ComrakRenderPlugins {
            codefence_syntax_highlighter: Some(SYNTECT_ADAPTOR.get().unwrap()),
        },
    }
}

static SYNTECT_ADAPTOR: OnceCell<SyntectAdapter> = OnceCell::new();

const COMRAK_OPTIONS: ComrakOptions = ComrakOptions {
    extension: ComrakExtensionOptions {
        strikethrough: true,
        table: true,
        tasklist: true,
        superscript: true,
        footnotes: true,
        tagfilter: false,
        autolink: false,
        header_ids: None,
        description_lists: false,
        front_matter_delimiter: None,
    },
    parse: ComrakParseOptions {
        smart: true,
        default_info_string: None,
    },
    render: ComrakRenderOptions {
        unsafe_: true,
        hardbreaks: false,
        github_pre_lang: false,
        width: 0,
        escape: false,
    },
};

#[cfg(test)]
mod tests {
    use super::{initialize_markdown, markdown};

    #[test]
    fn basic_markdown() {
        initialize_markdown();
        assert!(markdown("**Hello, world!**").contains("<p><strong>Hello, world!</strong></p>"));
    }
}
