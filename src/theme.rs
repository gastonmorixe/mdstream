use clap::ValueEnum;
use std::fmt;
use std::io::Cursor;
use std::str::FromStr;
use syntect::highlighting::{
    Color, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSet, ThemeSettings,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CodeTheme {
    #[value(name = "mdstream")]
    Mdstream,
    #[value(name = "catppuccin-mocha", alias = "Catppuccin Mocha")]
    CatppuccinMocha,
    #[value(name = "sublime-snazzy", alias = "Sublime Snazzy")]
    SublimeSnazzy,
    #[value(name = "dracula", alias = "Dracula")]
    Dracula,
    #[value(name = "inspired-github", alias = "InspiredGitHub")]
    InspiredGitHub,
    #[value(name = "solarized-dark", alias = "Solarized (dark)")]
    SolarizedDark,
    #[value(name = "solarized-light", alias = "Solarized (light)")]
    SolarizedLight,
    #[value(name = "base16-eighties-dark", alias = "base16-eighties.dark")]
    Base16EightiesDark,
    #[value(name = "base16-mocha-dark", alias = "base16-mocha.dark")]
    Base16MochaDark,
    #[value(name = "base16-ocean-dark", alias = "base16-ocean.dark")]
    Base16OceanDark,
    #[value(name = "base16-ocean-light", alias = "base16-ocean.light")]
    Base16OceanLight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PaletteColor {
    #[value(name = "h1")]
    H1,
    #[value(name = "h2")]
    H2,
    #[value(name = "h3")]
    H3,
    #[value(name = "h4")]
    H4,
    #[value(name = "h5")]
    H5,
    #[value(name = "h6")]
    H6,
}

pub const DEFAULT_CODE_THEME: CodeTheme = CodeTheme::Mdstream;
pub const DEFAULT_SHOW_CODE_BACKGROUND: bool = false;
pub const DEFAULT_INLINE_CODE_COLOR: PaletteColor = PaletteColor::H5;

const THEME_NAMES: &[&str] = &[
    "mdstream",
    "catppuccin-mocha",
    "sublime-snazzy",
    "dracula",
    "inspired-github",
    "solarized-dark",
    "solarized-light",
    "base16-eighties-dark",
    "base16-mocha-dark",
    "base16-ocean-dark",
    "base16-ocean-light",
];
const PALETTE_COLOR_NAMES: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];

const CATPPUCCIN_MOCHA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/themes/Catppuccin Mocha.tmTheme"
));
const SUBLIME_SNAZZY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/themes/Sublime Snazzy.tmTheme"
));
const DRACULA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/themes/Dracula.tmTheme"
));

impl CodeTheme {
    pub const fn theme_key(self) -> &'static str {
        match self {
            Self::Mdstream => "mdstream",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::SublimeSnazzy => "Sublime Snazzy",
            Self::Dracula => "Dracula",
            Self::InspiredGitHub => "InspiredGitHub",
            Self::SolarizedDark => "Solarized (dark)",
            Self::SolarizedLight => "Solarized (light)",
            Self::Base16EightiesDark => "base16-eighties.dark",
            Self::Base16MochaDark => "base16-mocha.dark",
            Self::Base16OceanDark => "base16-ocean.dark",
            Self::Base16OceanLight => "base16-ocean.light",
        }
    }

    pub const fn cli_name(self) -> &'static str {
        match self {
            Self::Mdstream => "mdstream",
            Self::CatppuccinMocha => "catppuccin-mocha",
            Self::SublimeSnazzy => "sublime-snazzy",
            Self::Dracula => "dracula",
            Self::InspiredGitHub => "inspired-github",
            Self::SolarizedDark => "solarized-dark",
            Self::SolarizedLight => "solarized-light",
            Self::Base16EightiesDark => "base16-eighties-dark",
            Self::Base16MochaDark => "base16-mocha-dark",
            Self::Base16OceanDark => "base16-ocean-dark",
            Self::Base16OceanLight => "base16-ocean-light",
        }
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        <Self as ValueEnum>::from_str(input, false)
    }

    pub fn all_names() -> &'static [&'static str] {
        THEME_NAMES
    }

    pub fn all_names_csv() -> String {
        THEME_NAMES.join(", ")
    }
}

impl fmt::Display for CodeTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.cli_name())
    }
}

impl PaletteColor {
    pub const fn cli_name(self) -> &'static str {
        match self {
            Self::H1 => "h1",
            Self::H2 => "h2",
            Self::H3 => "h3",
            Self::H4 => "h4",
            Self::H5 => "h5",
            Self::H6 => "h6",
        }
    }

    pub const fn ansi_escape(self) -> &'static str {
        match self {
            Self::H1 => "\x1b[38;2;255;100;100m",
            Self::H2 => "\x1b[38;2;255;170;80m",
            Self::H3 => "\x1b[38;2;100;220;100m",
            Self::H4 => "\x1b[38;2;100;180;255m",
            Self::H5 => "\x1b[38;2;180;140;255m",
            Self::H6 => "\x1b[38;2;200;120;180m",
        }
    }

    pub const fn for_heading_level(level: usize) -> Self {
        match level {
            1 => Self::H1,
            2 => Self::H2,
            3 => Self::H3,
            4 => Self::H4,
            5 => Self::H5,
            _ => Self::H6,
        }
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        <Self as ValueEnum>::from_str(input, false)
    }

    pub fn all_names() -> &'static [&'static str] {
        PALETTE_COLOR_NAMES
    }

    pub fn all_names_csv() -> String {
        PALETTE_COLOR_NAMES.join(", ")
    }
}

impl fmt::Display for PaletteColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.cli_name())
    }
}

pub fn load_theme_set() -> ThemeSet {
    let mut theme_set = ThemeSet::load_defaults();
    insert_tmtheme(
        &mut theme_set,
        CodeTheme::CatppuccinMocha.theme_key(),
        CATPPUCCIN_MOCHA,
    );
    insert_tmtheme(
        &mut theme_set,
        CodeTheme::SublimeSnazzy.theme_key(),
        SUBLIME_SNAZZY,
    );
    insert_tmtheme(&mut theme_set, CodeTheme::Dracula.theme_key(), DRACULA);
    theme_set
        .themes
        .insert(CodeTheme::Mdstream.theme_key().to_owned(), mdstream_theme());
    theme_set
}

fn insert_tmtheme(theme_set: &mut ThemeSet, name: &str, bytes: &[u8]) {
    let mut cursor = Cursor::new(bytes);
    let theme = ThemeSet::load_from_reader(&mut cursor).expect("embedded theme should parse");
    theme_set.themes.insert(name.to_owned(), theme);
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r, g, b, a: 0xFF }
}

fn style(scope: &str, foreground: Color) -> ThemeItem {
    style_with(scope, Some(foreground), None)
}

fn style_with(scope: &str, foreground: Option<Color>, font_style: Option<FontStyle>) -> ThemeItem {
    ThemeItem {
        scope: ScopeSelectors::from_str(scope).expect("theme scope selector should parse"),
        style: StyleModifier {
            foreground,
            background: None,
            font_style,
        },
    }
}

fn mdstream_theme() -> Theme {
    let base = rgb(232, 238, 250);
    let comment = rgb(113, 123, 148);
    let keyword = rgb(255, 97, 172);
    let string = rgb(120, 227, 140);
    let number = rgb(255, 183, 94);
    let function = rgb(96, 214, 255);
    let type_name = rgb(123, 167, 255);
    let constant = rgb(190, 132, 255);
    let punctuation = rgb(138, 216, 208);
    let parameter = rgb(255, 134, 112);
    let tag = rgb(255, 152, 82);
    let preprocessor = rgb(255, 211, 92);

    Theme {
        name: Some("mdstream".to_owned()),
        author: Some("mdstream".to_owned()),
        settings: ThemeSettings {
            foreground: Some(base),
            background: Some(Color::BLACK),
            caret: Some(base),
            line_highlight: Some(rgb(18, 18, 24)),
            gutter: Some(Color::BLACK),
            gutter_foreground: Some(comment),
            selection: Some(rgb(38, 40, 54)),
            selection_foreground: Some(base),
            active_guide: Some(rgb(52, 58, 74)),
            guide: Some(rgb(32, 36, 48)),
            ..ThemeSettings::default()
        },
        scopes: vec![
            style_with(
                "comment, punctuation.definition.comment",
                Some(comment),
                Some(FontStyle::ITALIC),
            ),
            style("string, punctuation.definition.string", string),
            style("constant.character.escape", keyword),
            style(
                "constant.numeric, constant.language.boolean, constant.language, variable.other.constant, constant.other",
                number,
            ),
            style(
                "keyword, keyword.control, keyword.operator.word, storage, storage.modifier",
                keyword,
            ),
            style(
                "keyword.operator, punctuation.accessor, punctuation.definition.generic, punctuation.separator, punctuation.separator.key-value, punctuation.terminator",
                punctuation,
            ),
            style(
                "entity.name.function, meta.function-call, support.function, support.function.misc, variable.function",
                function,
            ),
            style(
                "entity.name.class, entity.name.type, entity.name.enum, entity.name.struct, support.class, support.type, storage.type",
                type_name,
            ),
            style("entity.name.namespace, entity.name.module", constant),
            style_with(
                "variable.parameter",
                Some(parameter),
                Some(FontStyle::ITALIC),
            ),
            style(
                "entity.name.tag, punctuation.definition.tag, entity.other.attribute-name",
                tag,
            ),
            style(
                "meta.preprocessor, keyword.control.import, keyword.control.from",
                preprocessor,
            ),
            style(
                "support.constant, support.variable, support.other.constant",
                constant,
            ),
            style("markup.bold", number),
            style("markup.italic", keyword),
            style("markup.quote, markup.raw.inline", function),
            style("invalid, invalid.illegal", rgb(255, 93, 122)),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_names_are_stable_and_parseable() {
        assert_eq!(DEFAULT_CODE_THEME.cli_name(), "mdstream");
        assert_eq!(
            CodeTheme::parse("base16-ocean.dark").unwrap(),
            CodeTheme::Base16OceanDark
        );
        assert_eq!(
            CodeTheme::parse("Solarized (dark)").unwrap(),
            CodeTheme::SolarizedDark
        );
        assert_eq!(
            CodeTheme::parse("Catppuccin Mocha").unwrap(),
            CodeTheme::CatppuccinMocha
        );
        assert_eq!(CodeTheme::all_names().len(), 11);
    }

    #[test]
    fn palette_color_names_are_stable_and_parseable() {
        assert_eq!(DEFAULT_INLINE_CODE_COLOR.cli_name(), "h5");
        assert_eq!(PaletteColor::parse("h1").unwrap(), PaletteColor::H1);
        assert_eq!(PaletteColor::parse("h6").unwrap(), PaletteColor::H6);
        assert_eq!(
            PaletteColor::all_names(),
            &["h1", "h2", "h3", "h4", "h5", "h6"]
        );
        assert_eq!(
            PaletteColor::for_heading_level(5).ansi_escape(),
            "\x1b[38;2;180;140;255m"
        );
    }

    #[test]
    fn theme_set_contains_embedded_and_custom_themes() {
        let themes = load_theme_set();
        assert!(themes.themes.contains_key(CodeTheme::Mdstream.theme_key()));
        assert!(
            themes
                .themes
                .contains_key(CodeTheme::CatppuccinMocha.theme_key())
        );
        assert!(
            themes
                .themes
                .contains_key(CodeTheme::SublimeSnazzy.theme_key())
        );
        assert!(themes.themes.contains_key(CodeTheme::Dracula.theme_key()));
        assert!(
            themes
                .themes
                .contains_key(CodeTheme::InspiredGitHub.theme_key())
        );
    }
}
