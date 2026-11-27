//! Strings shown by the native parts of the app. The window has its own
//! translations in `src/app/i18n`.

pub struct Text {
    pub open: &'static str,
    pub stop: &'static str,
    pub quit: &'static str,
    pub idle_tooltip: &'static str,
    pub live_tooltip: &'static str,
}

const ENGLISH: Text = Text {
    open: "Open Telai",
    stop: "Stop streaming",
    quit: "Quit",
    idle_tooltip: "Telai",
    live_tooltip: "Telai: streaming",
};

const PORTUGUESE: Text = Text {
    open: "Abrir Telai",
    stop: "Parar transmissão",
    quit: "Sair",
    idle_tooltip: "Telai",
    live_tooltip: "Telai: transmitindo",
};

/// Picks the translation for the system language.
pub fn text() -> &'static Text {
    let language = sys_locale::get_locale().unwrap_or_default();

    if language.to_lowercase().starts_with("pt") {
        &PORTUGUESE
    } else {
        &ENGLISH
    }
}
