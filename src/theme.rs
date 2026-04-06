use clap::ValueEnum;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CodeTheme {
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

pub const DEFAULT_CODE_THEME: CodeTheme = CodeTheme::Base16OceanDark;

const THEME_NAMES: &[&str] = &[
    "inspired-github",
    "solarized-dark",
    "solarized-light",
    "base16-eighties-dark",
    "base16-mocha-dark",
    "base16-ocean-dark",
    "base16-ocean-light",
];

impl CodeTheme {
    pub const fn syntect_name(self) -> &'static str {
        match self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_names_are_stable_and_parseable() {
        assert_eq!(DEFAULT_CODE_THEME.cli_name(), "base16-ocean-dark");
        assert_eq!(
            CodeTheme::parse("base16-ocean.dark").unwrap(),
            CodeTheme::Base16OceanDark
        );
        assert_eq!(
            CodeTheme::parse("Solarized (dark)").unwrap(),
            CodeTheme::SolarizedDark
        );
        assert_eq!(CodeTheme::all_names().len(), 7);
    }
}
