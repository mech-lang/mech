use std::env;

pub(super) const NOFUN_ENV: &str = "MECH_NOFUN";
pub(super) const REPL_STYLE_ENV: &str = "MECH_REPL_STYLE";
pub(super) const REPL_QUIET_ENV: &str = "MECH_REPL_QUIET";
pub(super) const REPL_MAX_ELEMENTS_ENV: &str = "MECH_REPL_MAX_ELEMENTS";
pub(super) const QUIET_ENV: &str = "MECH_QUIET";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReplRenderMode {
    Rich,
    Plain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ReplUi {
    mode: ReplRenderMode,
    color: bool,
    #[cfg(feature = "mika")]
    animation: bool,
    quiet: bool,
    value_element_limit: usize,
}

impl ReplUi {
    pub(super) fn from_environment(nofun_flag: bool, quiet_flag: bool) -> Self {
        Self::resolve(EnvironmentSettings {
            nofun_flag,
            quiet_flag,
            mech_nofun: env::var(NOFUN_ENV).ok(),
            repl_style: env::var(REPL_STYLE_ENV).ok(),
            repl_quiet: env::var(REPL_QUIET_ENV).ok(),
            repl_max_elements: env::var(REPL_MAX_ELEMENTS_ENV).ok(),
            mech_quiet: env::var(QUIET_ENV).ok(),
            term: env::var("TERM").ok(),
            no_color: env::var_os("NO_COLOR").is_some(),
            clicolor: env::var("CLICOLOR").ok(),
            #[cfg(feature = "mika")]
            ci: env::var_os("CI").is_some(),
        })
    }

    #[cfg(test)]
    pub(super) const fn rich() -> Self {
        Self {
            mode: ReplRenderMode::Rich,
            color: true,
            #[cfg(feature = "mika")]
            animation: true,
            quiet: false,
            value_element_limit: mech_runtime::DEFAULT_REPL_VALUE_ELEMENT_LIMIT,
        }
    }

    #[cfg(test)]
    pub(super) const fn plain() -> Self {
        Self {
            mode: ReplRenderMode::Plain,
            color: false,
            #[cfg(feature = "mika")]
            animation: false,
            quiet: false,
            value_element_limit: mech_runtime::DEFAULT_REPL_VALUE_ELEMENT_LIMIT,
        }
    }

    pub(super) const fn mode(self) -> ReplRenderMode {
        self.mode
    }

    pub(super) const fn is_plain(self) -> bool {
        matches!(self.mode, ReplRenderMode::Plain)
    }

    pub(super) const fn color(self) -> bool {
        self.color
    }

    #[cfg(feature = "mika")]
    pub(super) const fn animation(self) -> bool {
        self.animation
    }

    pub(super) const fn quiet(self) -> bool {
        self.quiet
    }

    pub(super) const fn value_element_limit(self) -> usize {
        self.value_element_limit
    }

    fn resolve(settings: EnvironmentSettings) -> Self {
        let style = settings.repl_style.as_deref().map(str::trim);
        let explicit_rich = style.is_some_and(|style| {
            style.eq_ignore_ascii_case("rich") || style.eq_ignore_ascii_case("fun")
        });
        let explicit_plain = style.is_some_and(|style| {
            style.eq_ignore_ascii_case("plain") || style.eq_ignore_ascii_case("nofun")
        });
        let plain = settings.nofun_flag
            || settings.mech_nofun.as_deref().is_some_and(truthy)
            || explicit_plain
            || (!explicit_rich
                && settings
                    .term
                    .as_deref()
                    .is_some_and(|term| term.eq_ignore_ascii_case("dumb")));
        let quiet = settings.quiet_flag
            || settings.repl_quiet.as_deref().is_some_and(truthy)
            || settings.mech_quiet.as_deref().is_some_and(truthy);
        let value_element_limit = settings
            .repl_max_elements
            .as_deref()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(mech_runtime::DEFAULT_REPL_VALUE_ELEMENT_LIMIT);
        if plain {
            return Self {
                mode: ReplRenderMode::Plain,
                color: false,
                #[cfg(feature = "mika")]
                animation: false,
                quiet,
                value_element_limit,
            };
        }

        Self {
            mode: ReplRenderMode::Rich,
            color: !settings.no_color && settings.clicolor.as_deref() != Some("0"),
            #[cfg(feature = "mika")]
            animation: !settings.ci,
            quiet,
            value_element_limit,
        }
    }
}

#[derive(Debug, Default)]
struct EnvironmentSettings {
    nofun_flag: bool,
    quiet_flag: bool,
    mech_nofun: Option<String>,
    repl_style: Option<String>,
    repl_quiet: Option<String>,
    repl_max_elements: Option<String>,
    mech_quiet: Option<String>,
    term: Option<String>,
    no_color: bool,
    clicolor: Option<String>,
    #[cfg(feature = "mika")]
    ci: bool,
}

fn truthy(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nofun_sources_select_plain_mode() {
        for settings in [
            EnvironmentSettings {
                nofun_flag: true,
                ..EnvironmentSettings::default()
            },
            EnvironmentSettings {
                mech_nofun: Some("1".to_string()),
                ..EnvironmentSettings::default()
            },
            EnvironmentSettings {
                repl_style: Some("plain".to_string()),
                ..EnvironmentSettings::default()
            },
            EnvironmentSettings {
                term: Some("dumb".to_string()),
                ..EnvironmentSettings::default()
            },
        ] {
            assert_eq!(ReplUi::resolve(settings), ReplUi::plain());
        }
    }

    #[test]
    fn conventional_environment_controls_are_narrower_than_nofun() {
        let no_color = ReplUi::resolve(EnvironmentSettings {
            no_color: true,
            ..EnvironmentSettings::default()
        });
        assert_eq!(no_color.mode(), ReplRenderMode::Rich);
        assert!(!no_color.color());
        #[cfg(feature = "mika")]
        assert!(no_color.animation());

        #[cfg(feature = "mika")]
        let ci = ReplUi::resolve(EnvironmentSettings {
            ci: true,
            ..EnvironmentSettings::default()
        });
        #[cfg(feature = "mika")]
        assert_eq!(ci.mode(), ReplRenderMode::Rich);
        #[cfg(feature = "mika")]
        assert!(ci.color());
        #[cfg(feature = "mika")]
        assert!(!ci.animation());
    }

    #[test]
    fn quiet_flag_and_environment_do_not_change_visual_style() {
        for settings in [
            EnvironmentSettings {
                quiet_flag: true,
                ..EnvironmentSettings::default()
            },
            EnvironmentSettings {
                repl_quiet: Some("1".to_string()),
                ..EnvironmentSettings::default()
            },
            EnvironmentSettings {
                mech_quiet: Some("true".to_string()),
                ..EnvironmentSettings::default()
            },
        ] {
            let ui = ReplUi::resolve(settings);
            assert_eq!(ui.mode(), ReplRenderMode::Rich);
            assert!(ui.quiet());
        }
    }

    #[test]
    fn repl_element_limit_is_positive_and_defaults_to_portable_limit() {
        assert_eq!(
            ReplUi::resolve(EnvironmentSettings::default()).value_element_limit(),
            mech_runtime::DEFAULT_REPL_VALUE_ELEMENT_LIMIT,
        );
        assert_eq!(
            ReplUi::resolve(EnvironmentSettings {
                repl_max_elements: Some("72".to_string()),
                ..EnvironmentSettings::default()
            })
            .value_element_limit(),
            72,
        );
        for invalid in ["0", "-1", "many"] {
            assert_eq!(
                ReplUi::resolve(EnvironmentSettings {
                    repl_max_elements: Some(invalid.to_string()),
                    ..EnvironmentSettings::default()
                })
                .value_element_limit(),
                mech_runtime::DEFAULT_REPL_VALUE_ELEMENT_LIMIT,
            );
        }
    }
}
