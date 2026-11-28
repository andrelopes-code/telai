//! Strings shown by the native parts of the app. The window has its own
//! translations in `src/app/i18n`.

pub struct Text {
    pub open: &'static str,
    pub stop: &'static str,
    pub quit: &'static str,
    pub idle_tooltip: &'static str,
    pub live_tooltip: &'static str,
    /// Names a display that reports no model name: "Screen 2".
    pub screen: &'static str,
}

const ENGLISH: Text = Text {
    open: "Open Telai",
    stop: "Stop streaming",
    quit: "Quit",
    idle_tooltip: "Telai",
    live_tooltip: "Telai: streaming",
    screen: "Screen",
};

const PORTUGUESE: Text = Text {
    open: "Abrir Telai",
    stop: "Parar transmissão",
    quit: "Sair",
    idle_tooltip: "Telai",
    live_tooltip: "Telai: transmitindo",
    screen: "Tela",
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
