use clap::ValueEnum;

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ThemeOpt {
    Light,
    Dark1,
    Dark2,
    Tan,
    DarkTan,
    Marine,
    Blueish,
    Nord,
    HighContrast,
    Forest,
    SolarizedLight,
    GruvboxLight,
    GruvboxDark,
    Dracula,
    PurpleDusk,
    Monokai,
    Cyberpunk,
    SolarizedDark,
    MaterialDark,
    OceanicNext,
    Minimalist,
    Autumn,
    Mint,
    Vintage,
    Gray,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "PascalCase")]
pub enum SchemeOpt {
    Fleet1,
    Fleet2,
}

pub(crate) const DEFAULT_THEME: ThemeOpt = ThemeOpt::Dark2;
pub(crate) const DEFAULT_SCHEME: SchemeOpt = SchemeOpt::Fleet1;

pub struct Style {
    pub theme: ThemeOpt,
    pub scheme: SchemeOpt,
}

pub fn resolve(
    cli_theme: Option<ThemeOpt>,
    cli_scheme: Option<SchemeOpt>,
    cfg_theme: Option<ThemeOpt>,
    cfg_scheme: Option<SchemeOpt>,
) -> Style {
    let theme = cli_theme.or(cfg_theme).unwrap_or(DEFAULT_THEME);
    let scheme = cli_scheme.or(cfg_scheme).unwrap_or(DEFAULT_SCHEME);
    Style { theme, scheme }
}

pub(crate) fn canonical_theme_name(t: ThemeOpt) -> String {
    t.to_possible_value()
        .map(|v| v.get_name().to_string())
        .unwrap_or_else(|| format!("{:?}", t))
}

pub(crate) fn canonical_scheme_name(s: SchemeOpt) -> String {
    s.to_possible_value()
        .map(|v| v.get_name().to_string())
        .unwrap_or_else(|| format!("{:?}", s))
}

pub(crate) fn valid_theme_names() -> Vec<String> {
    ThemeOpt::value_variants()
        .iter()
        .filter_map(|v| v.to_possible_value())
        .map(|v| v.get_name().to_string())
        .collect()
}

pub(crate) fn valid_scheme_names() -> Vec<String> {
    SchemeOpt::value_variants()
        .iter()
        .filter_map(|v| v.to_possible_value())
        .map(|v| v.get_name().to_string())
        .collect()
}

pub(crate) fn parse_scheme(v: &str) -> Option<SchemeOpt> {
    <SchemeOpt as ValueEnum>::from_str(v, true).ok()
}

pub(crate) fn parse_theme(v: &str) -> Option<ThemeOpt> {
    <ThemeOpt as ValueEnum>::from_str(v, true).ok()
}

#[cfg(feature = "gui")]
pub(crate) fn apply_theme(theme: Option<ThemeOpt>, scheme: Option<SchemeOpt>) {
    use fltk_theme::{ColorTheme, SchemeType, WidgetScheme, color_themes};
    let theme = theme.unwrap_or(DEFAULT_THEME);
    let palette = match theme {
        ThemeOpt::Light => &color_themes::fleet::LIGHT,
        ThemeOpt::Dark1 => &color_themes::fleet::DARK1,
        ThemeOpt::Tan => &color_themes::fleet::TAN,
        ThemeOpt::DarkTan => &color_themes::fleet::DARK_TAN,
        ThemeOpt::Marine => &color_themes::fleet::MARINE,
        ThemeOpt::Blueish => &color_themes::fleet::BLUEISH,
        ThemeOpt::Nord => &color_themes::fleet::NORD,
        ThemeOpt::HighContrast => &color_themes::fleet::HIGH_CONTRAST,
        ThemeOpt::Forest => &color_themes::fleet::FOREST,
        ThemeOpt::SolarizedLight => &color_themes::fleet::SOLARIZED_LIGHT,
        ThemeOpt::GruvboxLight => &color_themes::fleet::GRUVBOX_LIGHT,
        ThemeOpt::Dark2 => &color_themes::fleet::DARK2,
        ThemeOpt::GruvboxDark => &color_themes::fleet::GRUVBOX_DARK,
        ThemeOpt::Dracula => &color_themes::fleet::DRACULA,
        ThemeOpt::PurpleDusk => &color_themes::fleet::PURPLE_DUSK,
        ThemeOpt::Monokai => &color_themes::fleet::MONOKAI,
        ThemeOpt::Cyberpunk => &color_themes::fleet::CYBERPUNK,
        ThemeOpt::SolarizedDark => &color_themes::fleet::SOLARIZED_DARK,
        ThemeOpt::MaterialDark => &color_themes::fleet::MATERIAL_DARK,
        ThemeOpt::OceanicNext => &color_themes::fleet::OCEANIC_NEXT,
        ThemeOpt::Minimalist => &color_themes::fleet::MINIMALIST,
        ThemeOpt::Autumn => &color_themes::fleet::AUTUMN,
        ThemeOpt::Mint => &color_themes::fleet::MINT,
        ThemeOpt::Vintage => &color_themes::fleet::VINTAGE,
        ThemeOpt::Gray => &color_themes::fleet::GRAY,
    };
    let color_theme = ColorTheme::new(palette);
    color_theme.apply();

    let scheme_ty = match scheme.unwrap_or(DEFAULT_SCHEME) {
        SchemeOpt::Fleet1 => SchemeType::Fleet1,
        SchemeOpt::Fleet2 => SchemeType::Fleet2,
    };
    let scheme = WidgetScheme::new(scheme_ty);
    scheme.apply();
}
